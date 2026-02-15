// Copyright (C) 2026  Jeff Shee
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

use std::{cell::RefCell, collections::HashMap, env, os::unix::process::CommandExt as _, rc::Rc};

use glib::Object;
use gtk::{gdk::Display, gio, glib, glib::Type, prelude::*};
use tracing::{debug, info, warn};

use crate::{
    model::{
        LaunchMode, MonitorListModelExt as _, Viewport, WallpaperConfig, WallpaperSource,
        WindowInfo, WindowLayout,
    },
    monitor_tracker::MonitorTracker,
    widget::{Renderer, RendererWidget, ClipBox},
    window::{HotaruApplicationWindow, Position},
};

glib::wrapper! {
    pub struct HotaruApplication(ObjectSubclass<imp::HotaruApplication>)
        @extends gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl HotaruApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        Object::builder()
            .property("application_id", application_id)
            .property("flags", flags)
            .build()
    }

    /// Build the UI and store active renderers in the shared state.
    ///
    /// The `renderers` parameter is a shared vec that is populated with the
    /// primary renderers created during this call. It is cleared first to
    /// remove any previously active renderers.
    pub fn build_ui(
        &self,
        config: &WallpaperConfig,
        use_clapper: bool,
        enable_graphics_offload: bool,
        content_fit: gtk::ContentFit,
        renderers: &Rc<RefCell<Vec<Renderer>>>,
        launch_mode: LaunchMode,
    ) {
        let monitor_map = MonitorTracker::monitors()
            .unwrap()
            .try_to_monitor_map()
            .unwrap();
        info!("Monitor map: {:#?}", monitor_map);

        let layout = WindowLayout::new(config, &monitor_map);
        info!("Window layout: {:#?}", layout);
        let mut primary_widgets = HashMap::new();

        layout.windows.iter().for_each(|window_info| {
            if let WindowInfo::Primary {
                monitor,
                window_x,
                window_y,
                window_width,
                window_height,
                window_title,
                wallpaper_type,
                wallpaper_source,
                viewport,
            } = window_info
            {
                let window = HotaruApplicationWindow::new(self, launch_mode);
                window.set_monitor_connector(monitor.as_str());
                window.set_position(Position {
                    x: *window_x,
                    y: *window_y,
                });
                window.set_size_request(*window_width, *window_height);
                debug!("window size request: {}x{}", window_width, window_height);
                window.set_title(Some(window_title));
                let renderer = match wallpaper_source {
                    WallpaperSource::Filepath { filepath } => Renderer::with_filepath(
                        filepath,
                        wallpaper_type,
                        use_clapper,
                        enable_graphics_offload,
                    ),
                    WallpaperSource::Uri { uri } => Renderer::with_uri(
                        uri,
                        wallpaper_type,
                        use_clapper,
                        enable_graphics_offload,
                    ),
                };
                if let Some(viewport) = viewport {
                    window.set_child(Some(&wrap_with_viewport(
                        renderer.widget(),
                        *window_width,
                        *window_height,
                        viewport,
                    )));
                } else {
                    window.set_child(Some(renderer.widget()));
                }
                window.present();
                renderer.play();
                primary_widgets.insert(monitor.to_string(), renderer);
            }
        });

        layout.windows.iter().for_each(|window_info| {
            if let WindowInfo::Clone {
                monitor,
                window_x,
                window_y,
                window_width,
                window_height,
                window_title,
                clone_source,
                viewport,
            } = window_info
            {
                let window = HotaruApplicationWindow::new(self, launch_mode);
                window.set_monitor_connector(monitor.as_str());
                window.set_position(Position {
                    x: *window_x,
                    y: *window_y,
                });
                window.set_size_request(*window_width, *window_height);
                debug!("window size request: {}x{}", window_width, window_height);
                window.set_title(Some(window_title));
                if let Some(primary_widget) = primary_widgets.get(clone_source) {
                    let widget = primary_widget.mirror(enable_graphics_offload, content_fit);
                    if let Some(viewport) = viewport {
                        window.set_child(Some(&wrap_with_viewport(
                            widget.upcast_ref(),
                            *window_width,
                            *window_height,
                            viewport,
                        )));
                    } else {
                        window.set_child(Some(&widget));
                    }
                }
                window.present();
            }
        });

        // Store renderers in shared state
        let mut shared = renderers.borrow_mut();
        shared.clear();
        shared.extend(primary_widgets.into_values());
    }
}

/// Wrap a widget so that only the portion visible through this monitor's
/// viewport is shown. The child is allocated at full canvas size and
/// translated by the viewport offset; rendering is clipped to the
/// window dimensions so the oversized child does not inflate the window.
fn wrap_with_viewport(
    child: &gtk::Widget,
    window_width: i32,
    window_height: i32,
    viewport: &Viewport,
) -> gtk::Widget {
    debug!(
        "Wrap with viewport: window size {}x{}, canvas size {}x{}, offset {}x{}",
        window_width,
        window_height,
        viewport.canvas_width,
        viewport.canvas_height,
        viewport.offset_x,
        viewport.offset_y
    );

    ClipBox::new(
        child,
        window_width,
        window_height,
        viewport.canvas_width,
        viewport.canvas_height,
        viewport.offset_x,
        viewport.offset_y,
    )
    .upcast()
}

/// If the current display is not X11, set `GDK_BACKEND=x11` and re-exec
/// the process so that GTK uses XWayland. This is required for X11Desktop
/// launch mode where we need X11 window type hints.
///
/// If already on X11, this is a no-op.
pub fn fallback_to_xwayland() {
    let display = Display::default().expect("Failed to get default display");
    let is_x11 = Type::from_name("GdkX11Display")
        .map(|x11_type| display.type_().is_a(x11_type))
        .unwrap_or(false);

    if !is_x11 {
        warn!("Display is not X11, re-executing with GDK_BACKEND=x11 for XWayland fallback");
        env::set_var("GDK_BACKEND", "x11");
        let args: Vec<String> = env::args().collect();
        let _error = std::process::Command::new(&args[0]).args(&args[1..]).exec();
        unreachable!("Failed to execute XWayland fallback");
    }
}

mod imp {
    use super::*;

    use gtk::{glib, subclass::prelude::*};

    #[derive(Default)]
    pub struct HotaruApplication;

    #[glib::object_subclass]
    impl ObjectSubclass for HotaruApplication {
        const NAME: &'static str = "HotaruApplication";
        type Type = super::HotaruApplication;
        type ParentType = gtk::Application;
    }

    impl ObjectImpl for HotaruApplication {}

    impl ApplicationImpl for HotaruApplication {
        fn command_line(&self, _command_line: &gio::ApplicationCommandLine) -> glib::ExitCode {
            // Just activate the application, we already handled the cli arguments with clap
            let app = self.obj();
            app.activate();
            glib::ExitCode::from(0)
        }
    }

    impl GtkApplicationImpl for HotaruApplication {}
}
