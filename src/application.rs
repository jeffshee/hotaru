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

use std::{cell::RefCell, collections::HashMap, env, os::unix::process::CommandExt as _, rc::Rc};

use glib::Object;
use gtk::{gdk::Display, gio, glib, glib::Type, prelude::*};
use tracing::{debug, info, warn};

use crate::{
    model::{
        LaunchMode, MonitorListModelExt as _, Viewport, WallpaperConfig, WallpaperSource,
        WallpaperType, WindowLayout, WindowRole,
    },
    monitor_tracker::MonitorTracker,
    renderer::{ClipBox, Renderer, RendererWidget},
    settings_watcher::RenderSettings,
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
        settings: &RenderSettings,
        renderers: &Rc<RefCell<Vec<Renderer>>>,
        launch_mode: LaunchMode,
    ) {
        let monitor_map = MonitorTracker::monitors().unwrap().monitor_map().unwrap();
        info!("Monitor map: {:#?}", monitor_map);

        let layout = WindowLayout::new(config, &monitor_map);
        info!("Window layout: {:#?}", layout);
        let mut primary_widgets = HashMap::new();

        // The layout orders primaries before clones, so a clone's source
        // renderer is always in `primary_widgets` by the time we reach it.
        for info in &layout.windows {
            let window = HotaruApplicationWindow::new(self, launch_mode);
            window.set_monitor_connector(info.monitor.as_str());
            window.set_position(Position {
                x: info.geometry.x,
                y: info.geometry.y,
            });
            window.set_size_request(info.geometry.width, info.geometry.height);
            debug!(
                "window size request: {}x{}",
                info.geometry.width, info.geometry.height
            );
            window.set_title(Some(&info.title));

            let child: Option<gtk::Widget> = match &info.role {
                WindowRole::Primary {
                    wallpaper_type,
                    wallpaper_source,
                } => {
                    let renderer = build_renderer(wallpaper_type, wallpaper_source, settings);
                    renderer.set_content_fit(settings.content_fit);
                    renderer.set_volume(settings.volume);
                    renderer.set_mute(settings.mute);
                    let widget = renderer.widget().clone();
                    primary_widgets.insert(info.monitor.clone(), renderer);
                    Some(widget)
                }
                WindowRole::Clone { source } => primary_widgets.get(source).map(|primary| {
                    primary
                        .mirror(settings.enable_graphics_offload, settings.content_fit)
                        .upcast()
                }),
            };

            if let Some(child) = child {
                if let Some(viewport) = &info.viewport {
                    window.set_child(Some(&wrap_with_viewport(
                        &child,
                        info.geometry.width,
                        info.geometry.height,
                        viewport,
                    )));
                } else {
                    window.set_child(Some(&child));
                }
            }
            window.present();

            if matches!(info.role, WindowRole::Primary { .. }) {
                if let Some(renderer) = primary_widgets.get(&info.monitor) {
                    renderer.play();
                }
            }
        }

        // Store renderers in shared state
        let mut shared = renderers.borrow_mut();
        shared.clear();
        shared.extend(primary_widgets.into_values());
    }
}

/// Construct the renderer for a primary window's wallpaper source.
fn build_renderer(
    wallpaper_type: &WallpaperType,
    wallpaper_source: &WallpaperSource,
    settings: &RenderSettings,
) -> Renderer {
    if *wallpaper_type == WallpaperType::Wpe {
        // WPE packages resolve their real renderer from project.json,
        // so they take the whole source (filepath or workshop_id).
        return Renderer::with_wpe(
            wallpaper_source,
            settings.video_renderer,
            settings.enable_graphics_offload,
        );
    }
    match wallpaper_source {
        WallpaperSource::Filepath { filepath } => Renderer::with_filepath(
            filepath,
            wallpaper_type,
            settings.video_renderer,
            settings.enable_graphics_offload,
        ),
        WallpaperSource::Uri { uri } => Renderer::with_uri(
            uri,
            wallpaper_type,
            settings.video_renderer,
            settings.enable_graphics_offload,
        ),
        WallpaperSource::WorkshopId { workshop_id } => {
            warn!(
                "workshop_id ({}) requires wallpaper_type: wpe; showing blank",
                workshop_id
            );
            Renderer::with_uri(
                "about:blank",
                &WallpaperType::Web,
                settings.video_renderer,
                settings.enable_graphics_offload,
            )
        }
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
