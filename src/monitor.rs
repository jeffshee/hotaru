use glib::Object;
use gtk::gdk::Display;
use gtk::gdk::Monitor;
use gtk::gdk::Rectangle;
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

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub connector: String,
    pub geometry: Rectangle,
}

pub trait MonitorListModelExt {
    fn try_to_monitor_vec(&self) -> Result<Vec<Monitor>, MonitorError>;
    fn try_to_monitor_info_vec(&self) -> Result<Vec<MonitorInfo>, MonitorError>;
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

    fn try_to_monitor_info_vec(&self) -> Result<Vec<MonitorInfo>, MonitorError> {
        let monitor_info = self
            .try_to_monitor_vec()?
            .iter()
            .map(MonitorInfo::from)
            .collect();
        Ok(monitor_info)
    }
}

impl From<&Monitor> for MonitorInfo {
    fn from(monitor: &Monitor) -> Self {
        let connector = match monitor.connector() {
            Some(connector) => connector.to_string(),
            None => "Unknown".to_string(),
        };

        Self {
            connector,
            geometry: monitor.geometry(),
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

    pub fn primary_monitor_info() -> Option<MonitorInfo> {
        Self::monitors()
            .ok()
            .and_then(|model| model.try_to_monitor_info_vec().ok())
            .and_then(|monitors| {
                monitors
                    .iter()
                    .find(|m| m.connector == "eDP-1") // Built-in monitor
                    .map(Clone::clone)
                    .or_else(|| {
                        monitors
                            .iter()
                            .rev()
                            .max_by_key(|m| m.geometry.width() * m.geometry.height()) // The largest monitor
                            .map(Clone::clone)
                    })
            })
    }

    pub fn primary_monitor_connector() -> Option<String> {
        Self::primary_monitor_info().map(|monitor| monitor.connector)
    }
}

mod imp {
    use super::*;
    use glib::subclass::Signal;
    use gtk::subclass::prelude::*;
    use std::sync::OnceLock;

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
                            println!("Monitor changed: {:?}", monitors);
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
