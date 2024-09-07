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

glib::wrapper! {
    pub struct HotaruApplicationWindow(ObjectSubclass<imp::HotaruApplicationWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

const WINDOW_TITLE: &str = "Hotaru Renderer";

impl HotaruApplicationWindow {
    pub fn new(app: &HotaruApplication) -> Self {
        let (width, height) = (1920, 1080);

        let title = if app.use_gnome_hanabi_ext() {
            hanabi_window_title()
        } else {
            WINDOW_TITLE.to_string()
        };

        let decorated =
            !app.use_x11_desktop() || !app.use_wayland_layer_shell() || !app.use_gnome_hanabi_ext();

        Object::builder()
            .property("application", app)
            .property("title", title)
            .property("decorated", decorated)
            // .property("default_width", width)
            // .property("default_height", height)
            .property("width_request", width)
            .property("height_request", height)
            .build()
    }
}

fn hanabi_window_title() -> String {
    // FIXME: Dummy
    let application_id = "io.github.jeffshee.HotaruRenderer";
    let index = 0;
    let state = object! {
        position: [0, 0],
        keepAtBottom: true,
        keepMinimized: true,
        keepPosition: true,
    };
    let state_json = json::stringify(state);

    format!("@${application_id}!${state_json}|${index}")
}

mod imp {
    use super::*;
    use gtk::subclass::prelude::*;

    use crate::application::HotaruApplication;

    #[derive(Default)]
    pub struct HotaruApplicationWindow;

    #[glib::object_subclass]
    impl ObjectSubclass for HotaruApplicationWindow {
        const NAME: &'static str = "HotaruApplicationWindow";
        type Type = super::HotaruApplicationWindow;
        type ParentType = gtk::ApplicationWindow;
    }

    impl ObjectImpl for HotaruApplicationWindow {}

    impl WidgetImpl for HotaruApplicationWindow {
        fn realize(&self) {
            self.parent_realize();

            let window: glib::BorrowedObject<'_, super::HotaruApplicationWindow> = self.obj();
            let app: HotaruApplication = window.property("application");
            if app.use_x11_desktop() {
                set_x11_window_type_hint(&window);
            }
        }
    }

    impl WindowImpl for HotaruApplicationWindow {}

    impl ApplicationWindowImpl for HotaruApplicationWindow {}

    fn set_x11_window_type_hint(window: &super::HotaruApplicationWindow) {
        use gdk_x11::X11Surface;
        use x11rb::connection::Connection;
        use x11rb::protocol::xproto::{AtomEnum, ConnectionExt, PropMode};
        use x11rb::wrapper::ConnectionExt as _;

        if let Some(surface) = window.surface() {
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

                window.connect_map(move |_window| {
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
}
