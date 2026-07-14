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

//! Active-wallpaper state and the single rebuild path, shared by
//! standalone mode and the D-Bus daemon. Lives on the GLib main thread.

use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr as _;

use gtk::gio::ListModel;
use gtk::glib;
use gtk::prelude::*;
use tracing::{debug, info};

use crate::application::HotaruApplication;
use crate::model::{LaunchMode, MonitorListModelExt as _, WallpaperConfig};
use crate::monitor_tracker::MonitorTracker;
use crate::settings_watcher::SettingsWatcher;
use crate::widget::{Renderer, RendererWidget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Idle,
    Playing,
    Paused,
}

impl PlaybackState {
    pub fn as_str(&self) -> &'static str {
        match self {
            PlaybackState::Idle => "idle",
            PlaybackState::Playing => "playing",
            PlaybackState::Paused => "paused",
        }
    }
}

pub struct RendererState {
    pub app: HotaruApplication,
    pub renderers: Rc<RefCell<Vec<Renderer>>>,
    pub config: RefCell<Option<WallpaperConfig>>,
    pub launch_mode: RefCell<LaunchMode>,
    pub playback_state: RefCell<PlaybackState>,
    pub settings_watcher: SettingsWatcher,
    /// The daemon's main loop. `GApplication::quit()` only stops a loop
    /// started by `app.run()`, which daemon mode never calls, so Quit must
    /// stop this loop explicitly.
    pub main_loop: RefCell<Option<glib::MainLoop>>,
}

impl RendererState {
    pub fn new(app: HotaruApplication) -> Rc<Self> {
        let renderers = Rc::new(RefCell::new(Vec::new()));
        let settings_watcher = SettingsWatcher::new();
        settings_watcher.connect_runtime_settings(renderers.clone());

        Rc::new(Self {
            app,
            renderers,
            config: RefCell::new(None),
            launch_mode: RefCell::new(LaunchMode::default()),
            playback_state: RefCell::new(PlaybackState::Idle),
            settings_watcher,
            main_loop: RefCell::new(None),
        })
    }

    /// Rebuild the wallpaper whenever the monitors change or the
    /// video-renderer setting is switched (no-op while no wallpaper is
    /// active). Wires both modes' triggers in one place.
    pub fn watch_changes(self: &Rc<Self>, monitor_tracker: &MonitorTracker) {
        let state = self.clone();
        monitor_tracker.connect_closure(
            "monitor-changed",
            false,
            glib::closure_local!(move |_monitor_tracker: MonitorTracker, list: ListModel| {
                let monitor_map = list.try_to_monitor_map().unwrap();
                debug!("monitor changed: {:?}", monitor_map);
                state.rebuild_ui();
            }),
        );

        let state = self.clone();
        self.settings_watcher.settings().connect_changed(
            Some("video-renderer"),
            move |_settings, _key| {
                info!("Video renderer setting changed, rebuilding");
                state.rebuild_ui();
            },
        );
    }

    /// Apply a wallpaper config: store it and (re)build the UI.
    pub fn apply(&self, config: &WallpaperConfig, launch_mode: LaunchMode) {
        // Update state before build_ui so the monitor-changed handler sees
        // the correct values.
        *self.launch_mode.borrow_mut() = launch_mode;
        *self.config.borrow_mut() = Some(config.clone());
        self.rebuild(config, launch_mode);
        *self.playback_state.borrow_mut() = PlaybackState::Playing;
    }

    /// D-Bus flavor of [`apply`](Self::apply): parse the JSON config and
    /// launch-mode strings, then persist them for auto-restore on the next
    /// daemon startup.
    pub fn apply_wallpaper(
        &self,
        config_json: &str,
        launch_mode_str: &str,
    ) -> Result<bool, String> {
        let config: WallpaperConfig =
            serde_json::from_str(config_json).map_err(|e| format!("Invalid config JSON: {}", e))?;
        let launch_mode = LaunchMode::from_str(launch_mode_str)
            .map_err(|e| format!("Invalid launch mode: {}", e))?;

        info!(
            "Applying wallpaper with mode {:?}, launch mode {:?}",
            config.mode, launch_mode
        );

        self.apply(&config, launch_mode);

        self.settings_watcher.set_last_wallpaper_config(config_json);
        self.settings_watcher.set_last_launch_mode(launch_mode_str);

        Ok(true)
    }

    /// Rebuild the wallpaper UI from the currently stored config, reading
    /// renderer/display settings fresh. No-op when no wallpaper is active.
    pub fn rebuild_ui(&self) {
        let Some(config) = self.config.borrow().clone() else {
            return;
        };
        let launch_mode = *self.launch_mode.borrow();
        self.rebuild(&config, launch_mode);
    }

    /// The single rebuild path: close all windows and rebuild with
    /// freshly-read settings.
    fn rebuild(&self, config: &WallpaperConfig, launch_mode: LaunchMode) {
        self.app.windows().into_iter().for_each(|w| w.close());

        let settings = self.settings_watcher.snapshot();
        self.app
            .build_ui(config, &settings, &self.renderers, launch_mode);

        // Defer settings application to avoid a GStreamer deadlock:
        // build_ui() starts pipeline state transitions via renderer.play(),
        // and setting properties (volume, mute) during the transition blocks
        // the main loop. An idle callback runs after the transition completes.
        let renderers = self.renderers.clone();
        glib::idle_add_local_once(move || {
            for renderer in renderers.borrow().iter() {
                renderer.set_volume(settings.volume);
                renderer.set_mute(settings.mute);
                renderer.set_content_fit(settings.content_fit);
            }
        });
    }

    pub fn disable_wallpaper(&self) -> bool {
        info!("Disabling wallpaper");

        for renderer in self.renderers.borrow().iter() {
            renderer.stop();
        }

        self.app.windows().into_iter().for_each(|w| w.close());

        self.renderers.borrow_mut().clear();
        *self.config.borrow_mut() = None;
        *self.playback_state.borrow_mut() = PlaybackState::Idle;

        // Clear persisted config
        self.settings_watcher.set_last_wallpaper_config("");
        self.settings_watcher.set_last_launch_mode("");

        true
    }

    pub fn pause(&self) -> bool {
        if *self.playback_state.borrow() != PlaybackState::Playing {
            return false;
        }
        info!("Pausing playback");
        for renderer in self.renderers.borrow().iter() {
            renderer.pause();
        }
        *self.playback_state.borrow_mut() = PlaybackState::Paused;
        true
    }

    pub fn resume(&self) -> bool {
        if *self.playback_state.borrow() != PlaybackState::Paused {
            return false;
        }
        info!("Resuming playback");
        for renderer in self.renderers.borrow().iter() {
            renderer.play();
        }
        *self.playback_state.borrow_mut() = PlaybackState::Playing;
        true
    }

    pub fn quit(&self) {
        info!("Quitting");
        self.app.quit();
        if let Some(main_loop) = self.main_loop.borrow().as_ref() {
            main_loop.quit();
        }
    }
}
