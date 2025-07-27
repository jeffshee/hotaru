use glib::Object;
use gtk::{
    gdk::{Display, Monitor},
    gio::ListModel,
    glib,
    prelude::*,
};

use crate::model::MonitorError;

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

impl Default for MonitorTracker {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use super::*;

    use std::sync::OnceLock;

    use glib::subclass::Signal;
    use gtk::subclass::prelude::*;
    use log::debug;

    use crate::model::MonitorListModelExt as _;

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
