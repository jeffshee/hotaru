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

mod cli;

use std::cell::RefCell;
use std::rc::Rc;

use clap::Parser as _;
use gtk::{
    gio::{self, ApplicationFlags, ListModel},
    glib,
    prelude::*,
};
use tracing::{debug, info};
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

use hotaru::dbus::{register_dbus_service, RendererState};
use hotaru::prelude::*;

use crate::cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("hotaru=info")))
        .init();
    info!("Hotaru started with args: {:#?}", cli);

    // The wallpaper-engine scene renderer needs a desktop GL 3.3 context,
    // but GDK prefers GLES on some EGL setups (notably NVIDIA), and a GL
    // GLArea context cannot share with a GLES display context. Steer GDK
    // away from GLES before it opens the display; HOTARU_ALLOW_GLES=1 opts
    // out (scene wallpapers will then fail where GDK picks GLES).
    #[cfg(feature = "scene")]
    if std::env::var_os("HOTARU_ALLOW_GLES").is_none() {
        let disable = match std::env::var("GDK_DISABLE") {
            Ok(value) if value.split(',').any(|v| v.trim() == "gles-api") => value,
            Ok(value) if !value.is_empty() => format!("{value},gles-api"),
            _ => "gles-api".to_string(),
        };
        std::env::set_var("GDK_DISABLE", disable);
    }

    gst::init().unwrap();
    // Register the statically linked gtk4paintablesink so hotaru does not
    // depend on the system's gst-plugins-rs package. If the system also
    // provides the plugin, the registry picks the newer version.
    gstgtk4::plugin_register_static().unwrap();
    gtk::init().unwrap();

    let mut app_flags = ApplicationFlags::HANDLES_COMMAND_LINE;
    if cli.daemon {
        // In daemon mode, prevent GApplication from claiming the bus name
        // so our zbus-based D-Bus service can own it instead.
        app_flags |= ApplicationFlags::NON_UNIQUE;
    }

    let app = HotaruApplication::new(APPLICATION_ID, &app_flags);

    if cli.daemon {
        // Daemon mode: register D-Bus service and wait for commands
        info!("Starting in daemon mode");

        let state = RendererState::new(app.clone());
        let monitor_tracker = MonitorTracker::new();

        // Register D-Bus service immediately (before app.run()) so it's
        // available as soon as the process starts. This is critical for
        // D-Bus activation: the caller expects the interface to be ready
        // shortly after the process is launched.
        register_dbus_service(state.clone());

        // In daemon mode, monitor changes trigger rebuild if a wallpaper is active
        let state_for_monitor = state.clone();
        monitor_tracker.connect_closure(
            "monitor-changed",
            false,
            glib::closure_local!(move |_monitor_tracker: MonitorTracker, list: ListModel| {
                let monitor_map = list.try_to_monitor_map().unwrap();
                debug!("monitor changed: {:?}", monitor_map);
                state_for_monitor.rebuild_ui();
            }),
        );

        // Switching the video renderer rebuilds the active wallpaper so the
        // change takes effect immediately.
        let state_for_renderer = state.clone();
        state.settings_watcher.settings().connect_changed(
            Some("video-renderer"),
            move |_settings, _key| {
                info!("Video renderer setting changed, rebuilding");
                state_for_renderer.rebuild_ui();
            },
        );

        // Register the GApplication so it can create windows, but hold it
        // so it stays alive even with no windows open.
        app.register(gio::Cancellable::NONE)?;
        let _hold_guard = app.hold();

        // Auto-restore last wallpaper if available
        let last_config = state.settings_watcher.last_wallpaper_config();
        let last_launch_mode = state.settings_watcher.last_launch_mode();
        if !last_config.is_empty() && !last_launch_mode.is_empty() {
            info!("Restoring last wallpaper on daemon startup");
            if let Err(e) = state.apply_wallpaper(&last_config, &last_launch_mode) {
                tracing::error!("Failed to restore last wallpaper: {}", e);
            }
        }

        // Run the GLib main loop directly. app.run() would exit immediately
        // with NON_UNIQUE because there's nothing keeping the run loop alive
        // before hold() takes effect.
        glib::MainLoop::new(None, false).run();
    } else {
        // Standalone mode: read config file and run immediately
        let settings_watcher = hotaru::settings_watcher::SettingsWatcher::new();
        let launch_mode = cli.launch_mode;

        // Handle XWayland fallback for X11Desktop mode
        if launch_mode == LaunchMode::X11Desktop {
            hotaru::application::fallback_to_xwayland();
        }

        let config_file = cli
            .config_file
            .ok_or_else(|| anyhow::anyhow!("--config is required when not in daemon mode"))?;
        let json = std::fs::read_to_string(&config_file)?;
        let config: WallpaperConfig = serde_json::from_str(&json)?;
        info!("Wallpaper config loaded: {:#?}", config);

        let renderers: Rc<RefCell<Vec<hotaru::widget::Renderer>>> =
            Rc::new(RefCell::new(Vec::new()));

        // Close all windows and rebuild with freshly-read settings.
        let rebuild = {
            let app = app.clone();
            let config = config.clone();
            let renderers = renderers.clone();
            let settings_watcher = hotaru::settings_watcher::SettingsWatcher::new();
            Rc::new(move || {
                app.windows().into_iter().for_each(|w| w.close());
                let video_renderer = settings_watcher.video_renderer();
                let enable_graphics_offload = settings_watcher.is_enable_graphics_offload();
                let content_fit = settings_watcher.content_fit();
                app.build_ui(
                    &config,
                    video_renderer,
                    enable_graphics_offload,
                    content_fit,
                    &renderers,
                    launch_mode,
                );
            })
        };

        let monitor_tracker = MonitorTracker::new();
        let rebuild_clone = rebuild.clone();
        monitor_tracker.connect_closure(
            "monitor-changed",
            false,
            glib::closure_local!(move |_monitor_tracker: MonitorTracker, list: ListModel| {
                let monitor_map = list.try_to_monitor_map().unwrap();
                debug!("monitor changed: {:?}", monitor_map);
                rebuild_clone();
            }),
        );

        // Switching the video renderer rebuilds the wallpaper so the change
        // takes effect immediately.
        let rebuild_clone = rebuild.clone();
        settings_watcher.settings().connect_changed(
            Some("video-renderer"),
            move |_settings, _key| {
                info!("Video renderer setting changed, rebuilding");
                rebuild_clone();
            },
        );

        let renderers_activate = renderers.clone();
        app.connect_activate(move |app| {
            let video_renderer = settings_watcher.video_renderer();
            let enable_graphics_offload = settings_watcher.is_enable_graphics_offload();
            let content_fit = settings_watcher.content_fit();
            app.build_ui(
                &config,
                video_renderer,
                enable_graphics_offload,
                content_fit,
                &renderers_activate,
                launch_mode,
            )
        });
        app.run();
    }

    Ok(())
}
