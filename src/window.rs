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
use gdk_x11::X11Surface;
use glib::Object;
use gtk::prelude::*;
use gtk::{gio, glib};
use json::{self, object};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{AtomEnum, ConfigureWindowAux, ConnectionExt, PropMode};
use x11rb::wrapper::ConnectionExt as _;

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
            .property("title", Some(WINDOW_TITLE))
            .property("decorated", false)
            .build()
    }

    fn set_x11_window_position(&self, x: i32, y: i32) {
        println!("set_x11_window_position: {x}, {y}");
        if let Some(surface) = self.surface() {
            if let Ok(x11_surface) = surface.downcast::<X11Surface>() {
                let xid = x11_surface.xid();
                println!("xid: {xid}");
                let (conn, _screen_num) = x11rb::connect(None).unwrap();
                let position = ConfigureWindowAux::new().x(x).y(y);

                let operation = move || {
                    conn.configure_window(xid as u32, &position)
                        .and_then(|_| conn.flush())
                        .unwrap_or_else(|e| eprintln!("Failed to position window: {}", e));
                };
                if self.is_mapped() {
                    operation();
                }
                self.connect_map(move |_window| {
                    operation();
                });
            } else {
                eprintln!("Failed to downcast Surface to X11Surface");
            }
        } else {
            eprintln!("Failed to get Surface");
        }
    }

    fn set_x11_window_type_hint(&self) {
        println!("set_x11_window_type_hint");
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

                let operation = move || {
                    conn.change_property32(
                        PropMode::REPLACE,
                        xid as u32,
                        net_wm_window_type,
                        AtomEnum::ATOM,
                        &[net_wm_window_type_desktop],
                    )
                    .and_then(|_| conn.flush())
                    .unwrap_or_else(|e| eprintln!("Failed to set window type hint: {}", e));
                };
                if self.is_mapped() {
                    operation();
                }
                self.connect_map(move |_window| {
                    operation();
                });
            } else {
                eprintln!("Failed to downcast Surface to X11Surface");
            }
        } else {
            eprintln!("Failed to get Surface");
        }
    }

    fn set_hanabi_window_title(&self) {
        // TODO: improve this
        let position = self.position();
        let state = object! {
            position: [position.x, position.y],
            keepAtBottom: true,
            keepMinimized: true,
            keepPosition: true,
        };
        let state_json = json::stringify(state);

        let title = format!("@{HANABI_APPLICATION_ID}!{state_json}");
        println!("title: {title}");
        self.set_title(Some(title.as_str()));
    }
}

#[derive(Clone, Copy, Debug, Default, glib::Boxed)]
#[boxed_type(name = "Position")]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

mod imp {
    use super::*;
    use glib::Properties;
    use gtk::{
        gdk::Display, style_context_add_provider_for_display, subclass::prelude::*, CssProvider,
        STYLE_PROVIDER_PRIORITY_APPLICATION,
    };
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::HotaruApplicationWindow)]
    pub struct HotaruApplicationWindow {
        #[property(get, construct_only)]
        window_type: RefCell<String>,
        #[property(get, set)]
        position: RefCell<Position>,
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

            // Black background
            let provider = CssProvider::new();
            provider.load_from_string(".black-bg { background-color: black; }");
            let display = Display::default().expect("Could not connect to a display");
            style_context_add_provider_for_display(
                &display,
                &provider,
                STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
            obj.set_css_classes(&["black-bg"]);

            let window_type = obj.window_type();
            println!("window_type: {window_type}");
            match window_type.as_str() {
                "x11-desktop" => {
                    obj.connect_realize(move |window| {
                        window.set_x11_window_type_hint();
                        let position = window.position();
                        window.set_x11_window_position(position.x, position.y);
                    });
                }
                "wayland-layer-shell" => {
                    todo!()
                }
                "gnome-ext-hanabi" => {
                    obj.connect_realize(move |window| {
                        window.set_hanabi_window_title();
                    });
                }
                "standalone" => {
                    obj.set_default_size(1920, 1080);

                    obj.connect_realize(move |window| {
                        window.set_decorated(true);
                    });
                }
                _ => {}
            }
        }
    }

    impl WidgetImpl for HotaruApplicationWindow {
        fn realize(&self) {
            self.parent_realize();
            println!("realize");
            let obj = self.obj();

            // Handle position changes after window realization
            obj.connect_position_notify(move |window| {
                println!("position_notify");
                let position = window.position();

                match window.window_type().as_str() {
                    "x11-desktop" => {
                        window.set_x11_window_position(position.x, position.y);
                    }
                    "gnome-ext-hanabi" => {
                        window.set_hanabi_window_title();
                    }
                    _ => {}
                }
            });
        }

        fn map(&self) {
            self.parent_map();
            println!("map");
        }
    }

    impl WindowImpl for HotaruApplicationWindow {}

    impl ApplicationWindowImpl for HotaruApplicationWindow {}
}
