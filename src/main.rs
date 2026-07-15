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
mod config;

use clap::Parser as _;
use gtk::{
    gio::{self, ApplicationFlags},
    glib,
    prelude::*,
};
use tracing::info;
use tracing_subscriber::{fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _, EnvFilter};

use hotaru::dbus::register_dbus_service;
use hotaru::prelude::*;
use hotaru::state::RendererState;

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
    #[cfg(feature = "wpe")]
    if std::env::var_os("HOTARU_ALLOW_GLES").is_none() {
        let disable = match std::env::var("GDK_DISABLE") {
            Ok(value) if value.split(',').any(|v| v.trim() == "gles-api") => value,
            Ok(value) if !value.is_empty() => format!("{value},gles-api"),
            _ => "gles-api".to_string(),
        };
        std::env::set_var("GDK_DISABLE", disable);
    }

    // WebKit's sandboxed WebProcess reads local <audio>/<video> files itself
    // (GStreamer filesrc), so web wallpapers get their directory granted into
    // the sandbox (see widget/web.rs) and the sandbox stays enabled. Escape
    // hatch: HOTARU_WEBKIT_SANDBOX=0 disables WebKit's sandbox entirely, for
    // wallpapers that read local media from outside their own directory.
    if std::env::var("HOTARU_WEBKIT_SANDBOX").as_deref() == Ok("0") {
        std::env::set_var("WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS", "1");
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

    // Both modes share the same state object and rebuild path: monitor
    // changes and video-renderer switches rebuild the active wallpaper.
    let state = RendererState::new(app.clone());
    let monitor_watcher = MonitorWatcher::new();
    state.watch_changes(&monitor_watcher);

    if cli.daemon {
        // Daemon mode: register D-Bus service and wait for commands
        info!("Starting in daemon mode");

        // Register D-Bus service immediately (before app.run()) so it's
        // available as soon as the process starts. This is critical for
        // D-Bus activation: the caller expects the interface to be ready
        // shortly after the process is launched.
        register_dbus_service(state.clone());

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
        // before hold() takes effect. Hand the loop to the D-Bus state so
        // the Quit method can stop it (app.quit() only affects app.run()).
        let main_loop = glib::MainLoop::new(None, false);
        state.main_loop.replace(Some(main_loop.clone()));
        main_loop.run();
    } else {
        // Standalone mode: read config file and run immediately. Load and
        // validate the config before the XWayland fallback, so a missing or
        // invalid --config fails fast instead of after the re-exec.
        let config_file = cli
            .config_file
            .ok_or_else(|| anyhow::anyhow!("--config is required unless --daemon"))?;
        let json = std::fs::read_to_string(&config_file)?;
        let config: WallpaperConfig = serde_json::from_str(&json)?;
        info!("Wallpaper config loaded: {:#?}", config);

        let launch_mode = cli.launch_mode;

        // Handle XWayland fallback for X11Desktop mode
        if launch_mode == LaunchMode::X11Desktop {
            hotaru::application::fallback_to_xwayland();
        }

        let state_for_activate = state.clone();
        app.connect_activate(move |_app| {
            state_for_activate.apply(&config, launch_mode);
        });
        app.run();
    }

    Ok(())
}
