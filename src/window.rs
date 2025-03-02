/* window.rs
 *
 * Copyright 2024 Jeff Shee
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */
use glib::Object;
use gtk::prelude::*;
use gtk::{gio, glib};
use json::{self, object};

use crate::application::HotaruApplication;

const WINDOW_TITLE: &str = "Hotaru Renderer";
const HANABI_APPLICATION_ID: &str = "io.github.jeffshee.HanabiRenderer";

pub enum WindowType {
    X11Desktop,
    WaylandLayerShell,
    GnomeExtHanabi,
    Standalone,
}

impl From<&WindowType> for glib::Value {
    fn from(value: &WindowType) -> Self {
        match value {
            WindowType::X11Desktop => glib::Value::from("x11-desktop"),
            WindowType::WaylandLayerShell => glib::Value::from("wayland-layer-shell"),
            WindowType::GnomeExtHanabi => glib::Value::from("gnome-ext-hanabi"),
            WindowType::Standalone => glib::Value::from("standalone"),
        }
    }
}

glib::wrapper! {
    pub struct HotaruApplicationWindow(ObjectSubclass<imp::HotaruApplicationWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl HotaruApplicationWindow {
    pub fn new(app: &HotaruApplication, window_type: &WindowType) -> Self {
        Object::builder()
            .property("application", app)
            .property("window_type", window_type)
            .build()
    }

    fn set_x11_window_type_hint(&self) {
        use gdk_x11::X11Surface;
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode};
        use x11rb::wrapper::ConnectionExt as _;

        if let Some(surface) = self.surface() {
            if let Ok(x11_surface) = surface.downcast::<X11Surface>() {
                let xid = x11_surface.xid();
                println!("xid: {xid}");
                let (conn, _screen_num) = x11rb::connect(None).unwrap();
                let net_wm_window_type = conn
                    .intern_atom(false, b"_NET_WM_WINDOW_TYPE")
                    .unwrap()
                    .reply()
                    .unwrap()
                    .atom;
                let net_wm_window_type_desktop = conn
                    .intern_atom(false, b"_NET_WM_WINDOW_TYPE_DESKTOP")
                    .unwrap()
                    .reply()
                    .unwrap()
                    .atom;
                conn.change_property32(
                    PropMode::REPLACE,
                    xid as u32,
                    net_wm_window_type,
                    AtomEnum::ATOM,
                    &[net_wm_window_type_desktop],
                )
                .unwrap();

                self.connect_map(move |_window| {
                    // Flush after the window is mapped, otherwise it will become a race condition
                    conn.flush().unwrap();
                });
            } else {
                eprintln!("Failed to downcast Surface to X11Surface");
            }
        } else {
            eprintln!("Failed to get Surface");
        }
    }

    fn set_hanabi_window_title(&self) {
        // TODO: Dummy
        let index = 0;
        let state = object! {
            position: [0, 0],
            keepAtBottom: true,
            keepMinimized: true,
            keepPosition: true,
        };
        let state_json = json::stringify(state);

        let title = format!("@${HANABI_APPLICATION_ID}!${state_json}|${index}");
        self.set_title(Some(title.as_str()));
    }
}

mod imp {
    use super::*;
    use glib::Properties;
    use gtk::subclass::prelude::*;
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::HotaruApplicationWindow)]
    pub struct HotaruApplicationWindow {
        #[property(get, construct_only)]
        window_type: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HotaruApplicationWindow {
        const NAME: &'static str = "HotaruApplicationWindow";
        type Type = super::HotaruApplicationWindow;
        type ParentType = gtk::ApplicationWindow;
    }

    #[glib::derived_properties]
    impl ObjectImpl for HotaruApplicationWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let window_type = obj.window_type();
            println!("window_type: {window_type}");
            match window_type.as_str() {
                "x11-desktop" => {
                    obj.set_decorated(false);
                    obj.set_size_request(1920, 1080);

                    obj.connect_realize(move |window| {
                        window.set_x11_window_type_hint();
                    });
                }
                "wayland-layer-shell" => {
                    obj.set_decorated(false);
                    obj.set_size_request(1920, 1080);
                    todo!()
                }
                "gnome-ext-hanabi" => {
                    obj.set_decorated(false);
                    obj.set_hanabi_window_title();
                    obj.set_size_request(1920, 1080);
                }
                "standalone" => {
                    obj.set_decorated(true);
                    obj.set_title(Some(WINDOW_TITLE));
                    obj.set_default_size(1920, 1080);
                }
                _ => {}
            }
        }
    }

    impl WidgetImpl for HotaruApplicationWindow {}

    impl WindowImpl for HotaruApplicationWindow {}

    impl ApplicationWindowImpl for HotaruApplicationWindow {}
}
