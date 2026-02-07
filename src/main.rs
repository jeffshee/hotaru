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
    gio::{ApplicationFlags, ListModel},
    glib,
    prelude::*,
};
use log::{debug, info};

use hotaru::dbus::{register_dbus_service, RendererState};
use hotaru::prelude::*;

use crate::cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let is_use_clapper = cli.is_use_clapper();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&cli.log_level))
        .init();
    info!("Hotaru started with args: {:#?}", cli);

    gst::init().unwrap();
    gtk::init().unwrap();

    // Read GSettings for startup-only settings, with CLI overrides
    let settings_watcher = hotaru::settings_watcher::SettingsWatcher::new();
    let is_enable_va = cli.is_enable_va() || settings_watcher.is_enable_va();
    let is_enable_nvsl = cli.is_enable_nvsl() || settings_watcher.is_enable_nvsl();
    setup_gst(is_enable_va, is_enable_nvsl);

    let app = HotaruApplication::new(
        APPLICATION_ID,
        &ApplicationFlags::HANDLES_COMMAND_LINE,
        cli.launch_mode,
    );

    if cli.daemon {
        // Daemon mode: register D-Bus service and wait for commands
        info!("Starting in daemon mode");

        let state = RendererState::new(app.clone(), is_use_clapper);
        let state_clone = state.clone();
        let monitor_tracker = MonitorTracker::new();

        app.connect_activate(move |_app| {
            info!("Daemon activated, registering D-Bus service");
            register_dbus_service(state_clone.clone());
        });

        // In daemon mode, monitor changes trigger rebuild if a wallpaper is active
        let state_for_monitor = state.clone();
        monitor_tracker.connect_closure(
            "monitor-changed",
            false,
            glib::closure_local!(move |_monitor_tracker: MonitorTracker, list: ListModel| {
                let monitor_map = list.try_to_monitor_map().unwrap();
                debug!("monitor changed: {:?}", monitor_map);
                if let Some(config) = state_for_monitor.config.borrow().as_ref() {
                    state_for_monitor
                        .app
                        .windows()
                        .into_iter()
                        .for_each(|w| w.close());
                    state_for_monitor.app.build_ui(
                        config,
                        state_for_monitor.use_clapper,
                        &state_for_monitor.renderers,
                    );
                    state_for_monitor
                        .settings_watcher
                        .apply_to_renderers(&state_for_monitor.renderers.borrow());
                }
            }),
        );

        app.run();
    } else {
        // Standalone mode: read config file and run immediately
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
                app_clone.build_ui(&config_clone, is_use_clapper, &renderers_clone);
            }),
        );

        let renderers_activate = renderers.clone();
        app.connect_activate(move |app| app.build_ui(&config, is_use_clapper, &renderers_activate));
        app.run();
    }

    Ok(())
}
