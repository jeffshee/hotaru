use glib::Object;
use gtk::gdk::prelude::MonitorExt as GdkMonitorExt;
use gtk::gdk::{Display, Monitor, Rectangle};
use gtk::gio::ListModel;
use gtk::glib;
use gtk::prelude::*;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("No display")]
    NoDisplay,
    #[error("ListModel error: {0}")]
    ListModel(String),
}

#[derive(Debug, Clone, PartialEq)]
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
                item.map_err(|e| MonitorError::ListModel(e.to_string()))?
                    .downcast::<Monitor>()
                    .map_err(|o| {
                        MonitorError::ListModel(format!(
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

pub trait MonitorExt: GdkMonitorExt {
    fn is_connector(&self, connector: &str) -> bool;
    fn area(&self) -> i32;
}

impl MonitorExt for Monitor {
    fn is_connector(&self, connector: &str) -> bool {
        self.connector().is_some_and(|c| c == connector)
    }

    fn area(&self) -> i32 {
        self.geometry().width() * self.geometry().height()
    }
}

glib::wrapper! {
    pub struct MonitorManager(ObjectSubclass<imp::MonitorManager>);
}

impl MonitorManager {
    pub fn new() -> Self {
        Object::new()
    }

    pub fn monitors() -> Result<ListModel, MonitorError> {
        Ok(Display::default()
            .ok_or(MonitorError::NoDisplay)?
            .monitors())
    }

    pub fn primary_monitor() -> Option<Monitor> {
        Self::monitors()
            .ok()
            .and_then(|model| model.try_to_monitor_vec().ok())
            .and_then(|monitors| {
                monitors
                    .iter()
                    .find(|m| m.is_connector("eDP-1")) // Built-in monitor
                    .map(Clone::clone)
                    .or_else(|| {
                        monitors
                            .iter()
                            .rev()
                            .max_by_key(|m| m.area()) // The largest monitor
                            .map(Clone::clone)
                    })
            })
    }

    pub fn primary_monitor_info() -> Option<MonitorInfo> {
        Self::primary_monitor().map(|monitor| MonitorInfo::from(&monitor))
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
    pub struct MonitorManager;

    #[glib::object_subclass]
    impl ObjectSubclass for MonitorManager {
        const NAME: &'static str = "MonitorManager";
        type Type = super::MonitorManager;
        type ParentType = Object;
    }

    impl ObjectImpl for MonitorManager {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let monitors = super::MonitorManager::monitors().unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            gtk::init().unwrap();
        });
    }

    #[test]
    fn test_monitors() {
        init();

        let monitors = MonitorManager::monitors();
        assert!(monitors.is_ok());

        let primary_monitor = MonitorManager::primary_monitor();
        assert!(primary_monitor.is_some());

        let primary_monitor_connector = MonitorManager::primary_monitor_connector();
        assert!(primary_monitor_connector.is_some());
        assert_ne!(
            primary_monitor_connector.clone().unwrap(),
            "Unknown".to_string()
        );
        println!("Primary monitor: {}", primary_monitor_connector.unwrap());

        let monitors = monitors.unwrap().try_to_monitor_info_vec();
        assert!(monitors.is_ok());
        let monitors = monitors.unwrap();
        assert_ne!(monitors.len(), 0);
        println!("Monitors: {:#?}", monitors)
    }
}
