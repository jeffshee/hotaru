use std::collections::HashMap;

use glib::Object;
use gtk::gdk::Display;
use gtk::gdk::Monitor;
use gtk::gio::ListModel;
use gtk::glib;
use gtk::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("No display")]
    NoDisplay,
    #[error("Monitor error: {0}")]
    MonitorError(String),
    #[error("ListModel error: {0}")]
    MonitorListModel(String),
}

pub type MonitorMap = HashMap<String, MonitorInfo>;

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

pub trait MonitorListModelExt {
    fn try_to_monitor_vec(&self) -> Result<Vec<Monitor>, MonitorError>;
    fn try_to_monitor_map(&self) -> Result<MonitorMap, MonitorError>;
}

impl MonitorListModelExt for ListModel {
    fn try_to_monitor_vec(&self) -> Result<Vec<Monitor>, MonitorError> {
        self.into_iter()
            .map(|item| {
                item.map_err(|e| MonitorError::MonitorListModel(e.to_string()))?
                    .downcast::<Monitor>()
                    .map_err(|o| {
                        MonitorError::MonitorListModel(format!(
                            "Failed to downcast object to gdk::Monitor, object: {:?}",
                            o
                        ))
                    })
            })
            .collect()
    }

    fn try_to_monitor_map(&self) -> Result<MonitorMap, MonitorError> {
        Ok(self
            .try_to_monitor_vec()?
            .iter()
            .map(|m| {
                let connector = match m.connector() {
                    Some(connector) => connector.to_string(),
                    None => "Unknown".to_string(),
                };

                (connector, m.into())
            })
            .collect())
    }
}

impl From<&Monitor> for MonitorInfo {
    fn from(monitor: &Monitor) -> Self {
        Self {
            x: monitor.geometry().x(),
            y: monitor.geometry().y(),
            height: monitor.geometry().height(),
            width: monitor.geometry().width(),
        }
    }
}

glib::wrapper! {
    pub struct MonitorTracker(ObjectSubclass<imp::MonitorTracker>);
}

impl MonitorTracker {
    pub fn new() -> Self {
        Object::new()
    }

    pub fn monitors() -> Result<ListModel, MonitorError> {
        Ok(Display::default()
            .ok_or(MonitorError::NoDisplay)?
            .monitors())
    }
}

mod imp {
    use super::*;

    use std::sync::OnceLock;

    use glib::subclass::Signal;
    use gtk::subclass::prelude::*;
    use log::debug;

    #[derive(Default)]
    pub struct MonitorTracker;

    #[glib::object_subclass]
    impl ObjectSubclass for MonitorTracker {
        const NAME: &'static str = "MonitorTracker";
        type Type = super::MonitorTracker;
        type ParentType = Object;
    }

    impl ObjectImpl for MonitorTracker {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let monitors = super::MonitorTracker::monitors().unwrap();
            monitors.connect_items_changed(glib::clone!(
                #[weak]
                obj,
                move |list, _, _, _| {
                    glib::idle_add_local_once(glib::clone!(
                        #[weak]
                        list,
                        move || {
                            let monitors: Vec<Monitor> = list.try_to_monitor_vec().unwrap();
                            debug!("monitor changed: {:?}", monitors);
                            obj.emit_by_name("monitor-changed", &[&list])
                        }
                    ));
                }
            ));
        }

        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("monitor-changed")
                    .param_types([ListModel::static_type()])
                    .build()]
            })
        }
    }
}
