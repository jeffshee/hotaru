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
    let is_use_clapper = cli.is_use_clapper();

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&cli.log_level)))
        .init();
    info!("Hotaru started with args: {:#?}", cli);

    gst::init().unwrap();
    gtk::init().unwrap();

    // Read GSettings for startup-only settings, with CLI overrides
    let settings_watcher = hotaru::settings_watcher::SettingsWatcher::new();
    let is_enable_va = cli.is_enable_va() || settings_watcher.is_enable_va();
    let is_enable_nvsl = cli.is_enable_nvsl() || settings_watcher.is_enable_nvsl();
    setup_gst(is_enable_va, is_enable_nvsl);

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

        let state = RendererState::new(app.clone(), is_use_clapper);
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
                if let Some(config) = state_for_monitor.config.borrow().as_ref() {
                    let launch_mode = *state_for_monitor.launch_mode.borrow();
                    state_for_monitor
                        .app
                        .windows()
                        .into_iter()
                        .for_each(|w| w.close());
                    state_for_monitor.app.build_ui(
                        config,
                        state_for_monitor.use_clapper,
                        &state_for_monitor.renderers,
                        launch_mode,
                    );
                    state_for_monitor
                        .settings_watcher
                        .apply_to_renderers(&state_for_monitor.renderers.borrow());
                }
            }),
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

        let app_clone = app.clone();
        let config_clone = config.clone();
        let renderers_clone = renderers.clone();
        let monitor_tracker = MonitorTracker::new();
        monitor_tracker.connect_closure(
            "monitor-changed",
            false,
            glib::closure_local!(move |_monitor_tracker: MonitorTracker, list: ListModel| {
                let monitor_map = list.try_to_monitor_map().unwrap();
                debug!("monitor changed: {:?}", monitor_map);
                app_clone.windows().into_iter().for_each(|w| w.close());
                app_clone.build_ui(&config_clone, is_use_clapper, &renderers_clone, launch_mode);
            }),
        );

        let renderers_activate = renderers.clone();
        app.connect_activate(move |app| {
            app.build_ui(&config, is_use_clapper, &renderers_activate, launch_mode)
        });
        app.run();
    }

    Ok(())
}
