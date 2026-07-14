// Copyright (C) 2026 Jeff Shee <jeffshee8969@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use glib::Object;
use gtk::{
    gdk::{Display, Monitor},
    gio::ListModel,
    glib,
    prelude::*,
};

use crate::model::MonitorError;

glib::wrapper! {
    pub struct MonitorWatcher(ObjectSubclass<imp::MonitorWatcher>);
}

impl MonitorWatcher {
    pub fn new() -> Self {
        Object::new()
    }

    pub fn monitors() -> Result<ListModel, MonitorError> {
        Ok(Display::default()
            .ok_or(MonitorError::NoDisplay)?
            .monitors())
    }
}

impl Default for MonitorWatcher {
    fn default() -> Self {
        Self::new()
    }
}

mod imp {
    use super::*;

    use std::sync::OnceLock;

    use glib::subclass::Signal;
    use gtk::subclass::prelude::*;
    use tracing::debug;

    use crate::model::MonitorListModelExt as _;

    #[derive(Default)]
    pub struct MonitorWatcher;

    #[glib::object_subclass]
    impl ObjectSubclass for MonitorWatcher {
        const NAME: &'static str = "MonitorWatcher";
        type Type = super::MonitorWatcher;
        type ParentType = Object;
    }

    impl ObjectImpl for MonitorWatcher {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();

            let monitors = super::MonitorWatcher::monitors().unwrap();
            monitors.connect_items_changed(glib::clone!(
                #[weak]
                obj,
                move |list, _, _, _| {
                    glib::idle_add_local_once(glib::clone!(
                        #[weak]
                        list,
                        move || {
                            let monitors: Vec<Monitor> = list.monitor_vec().unwrap();
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
