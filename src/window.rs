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

use gdk_x11::X11Surface;
use glib::Object;
use gtk::{gio, glib, prelude::*};
use gtk4_layer_shell::LayerShell;
use tracing::{debug, error};
use x11rb::{
    connection::Connection,
    protocol::xproto::{AtomEnum, ConfigureWindowAux, ConnectionExt, PropMode},
    wrapper::ConnectionExt as _,
};

use crate::{
    application::HotaruApplication,
    constants::WINDOW_TITLE,
    model::{HanabiParams, LaunchMode, MonitorListModelExt as _},
};

glib::wrapper! {
    pub struct HotaruApplicationWindow(ObjectSubclass<imp::HotaruApplicationWindow>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl HotaruApplicationWindow {
    pub fn new(app: &HotaruApplication, launch_mode: LaunchMode) -> Self {
        Object::builder()
            .property("application", app)
            .property("launch_mode", launch_mode)
            .property("title", Some(WINDOW_TITLE))
            .property("decorated", false)
            .build()
    }

    /// Run `operation` with an X11 connection and this window's xid.
    /// A no-op (with an error log) when the window has no X11 surface.
    fn with_x11_window(
        &self,
        operation: impl FnOnce(&x11rb::rust_connection::RustConnection, u32) -> anyhow::Result<()>,
    ) {
        let Some(surface) = self.surface() else {
            error!("Failed to get Surface");
            return;
        };
        let Ok(x11_surface) = surface.downcast::<X11Surface>() else {
            error!("Failed to downcast Surface to X11Surface");
            return;
        };
        let xid = x11_surface.xid() as u32;
        debug!("xid: {xid}");
        let conn = match x11rb::connect(None) {
            Ok((conn, _screen_num)) => conn,
            Err(e) => {
                error!("Failed to connect to X11: {}", e);
                return;
            }
        };
        if let Err(e) = operation(&conn, xid).and_then(|_| Ok(conn.flush()?)) {
            error!("X11 window operation failed: {}", e);
        }
    }

    fn set_x11_window_position(&self, x: i32, y: i32) {
        debug!("set_x11_window_position: {x}, {y}");
        self.with_x11_window(|conn, xid| {
            conn.configure_window(xid, &ConfigureWindowAux::new().x(x).y(y))?;
            Ok(())
        });
    }

    /// Apply the X11 desktop-window properties: the DESKTOP window type
    /// hint, zeroed `_GTK_FRAME_EXTENTS` (so Mutter draws no compositor-side
    /// shadow), and the window position. Called on map, and the position
    /// again whenever it changes.
    fn apply_x11_desktop_setup(&self) {
        debug!("apply_x11_desktop_setup");
        let position = self.position();
        self.with_x11_window(|conn, xid| {
            let atom = |name: &[u8]| -> anyhow::Result<u32> {
                Ok(conn.intern_atom(false, name)?.reply()?.atom)
            };
            conn.change_property32(
                PropMode::REPLACE,
                xid,
                atom(b"_NET_WM_WINDOW_TYPE")?,
                AtomEnum::ATOM,
                &[atom(b"_NET_WM_WINDOW_TYPE_DESKTOP")?],
            )?;
            conn.change_property32(
                PropMode::REPLACE,
                xid,
                atom(b"_GTK_FRAME_EXTENTS")?,
                AtomEnum::CARDINAL,
                &[0, 0, 0, 0],
            )?;
            conn.configure_window(xid, &ConfigureWindowAux::new().x(position.x).y(position.y))?;
            Ok(())
        });
    }

    fn set_hanabi_window_title(&self) {
        let position = self.position();
        let params = HanabiParams {
            position: [position.x, position.y],
            keep_at_bottom: true,
            keep_minimized: true,
            keep_position: true,
        };
        self.set_title(Some(&params.window_title()));
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
    use std::cell::{Cell, RefCell};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::HotaruApplicationWindow)]
    pub struct HotaruApplicationWindow {
        #[property(get, construct_only)]
        launch_mode: RefCell<LaunchMode>,
        #[property(get, set)]
        monitor_connector: RefCell<String>,
        #[property(get, set)]
        position: RefCell<Position>,
        pub(super) frame_count: Cell<u32>,
        pub(super) last_fps_time: Cell<i64>,
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

            match obj.launch_mode() {
                LaunchMode::X11Desktop => {
                    // Applied on every map: a remap starts from fresh X
                    // window state, so the hints must be set again.
                    obj.connect_map(|window| window.apply_x11_desktop_setup());
                }
                LaunchMode::WaylandLayerShell => {
                    obj.init_layer_shell();
                    obj.set_layer(gtk4_layer_shell::Layer::Background);
                    obj.set_anchor(gtk4_layer_shell::Edge::Left, true);
                    obj.set_anchor(gtk4_layer_shell::Edge::Right, true);
                    obj.set_anchor(gtk4_layer_shell::Edge::Top, true);
                    obj.set_anchor(gtk4_layer_shell::Edge::Bottom, true);
                    obj.set_exclusive_zone(-1);
                    obj.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::None);
                    obj.set_namespace(Some("hotaru-layer-shell"));

                    obj.connect_realize(move |window| {
                        let connector = window.monitor_connector();
                        let display = Display::default().expect("Could not connect to a display");
                        if let Ok(monitors) = display.monitors().monitor_vec() {
                            for monitor in &monitors {
                                if monitor.connector().as_deref() == Some(&connector) {
                                    window.set_monitor(Some(monitor));
                                    break;
                                }
                            }
                        }
                    });
                }
                LaunchMode::GnomeExtHanabi => {
                    obj.connect_realize(move |window| {
                        window.set_hanabi_window_title();
                    });
                }
                LaunchMode::Windowed => {
                    obj.connect_realize(move |window| {
                        window.set_decorated(true);
                    });
                }
            }
        }
    }

    impl WidgetImpl for HotaruApplicationWindow {
        fn realize(&self) {
            self.parent_realize();
            debug!("realize");
            let obj = self.obj();

            // Handle position changes after window realization
            obj.connect_position_notify(move |window| {
                debug!("position_notify");
                let position = window.position();

                match window.launch_mode() {
                    LaunchMode::X11Desktop => {
                        // While unmapped, the map handler applies the
                        // (property-stored) position instead.
                        if window.is_mapped() {
                            window.set_x11_window_position(position.x, position.y);
                        }
                    }
                    LaunchMode::GnomeExtHanabi => {
                        window.set_hanabi_window_title();
                    }
                    LaunchMode::WaylandLayerShell | LaunchMode::Windowed => {
                        // No position updates needed
                    }
                }
            });
        }

        fn map(&self) {
            self.parent_map();
            debug!("map");

            let obj = self.obj();
            obj.add_tick_callback(|window, frame_clock| {
                let imp = window.imp();
                imp.frame_count.set(imp.frame_count.get() + 1);
                let now = frame_clock.frame_time(); // microseconds
                let last = imp.last_fps_time.get();
                if last == 0 {
                    imp.last_fps_time.set(now);
                    return glib::ControlFlow::Continue;
                }
                if now - last >= 1_000_000 {
                    let frames = imp.frame_count.get();
                    let fps = frame_clock.fps();
                    let connector = imp.monitor_connector.borrow();
                    debug!("[{connector}] frames: {frames}, fps: {fps:.1}");
                    imp.frame_count.set(0);
                    imp.last_fps_time.set(now);
                }
                glib::ControlFlow::Continue
            });
        }
    }

    impl WindowImpl for HotaruApplicationWindow {}

    impl ApplicationWindowImpl for HotaruApplicationWindow {}
}
