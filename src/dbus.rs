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

use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr as _;
use std::sync::mpsc;

use gtk::glib;
use gtk::prelude::*;
use tracing::info;

use crate::application::HotaruApplication;
use crate::model::{LaunchMode, WallpaperConfig};
use crate::settings_watcher::SettingsWatcher;
use crate::widget::{Renderer, RendererWidget};

pub const DBUS_NAME: &str = "io.github.jeffshee.Hotaru";
pub const DBUS_PATH: &str = "/io/github/jeffshee/Hotaru";

// --- Commands sent from D-Bus thread to GLib main thread ---

enum Command {
    ApplyWallpaper {
        config_json: String,
        launch_mode: String,
        reply: mpsc::Sender<Result<bool, String>>,
    },
    DisableWallpaper {
        reply: mpsc::Sender<bool>,
    },
    Pause {
        reply: mpsc::Sender<bool>,
    },
    Resume {
        reply: mpsc::Sender<bool>,
    },
    Quit,
    GetState {
        reply: mpsc::Sender<String>,
    },
}

// --- RendererState: lives on the GLib main thread ---

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
        })
    }

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
            "D-Bus: Applying wallpaper with mode {:?}, launch mode {:?}",
            config.mode, launch_mode
        );

        // Update state before build_ui so monitor-changed handler sees correct values
        *self.launch_mode.borrow_mut() = launch_mode;
        *self.config.borrow_mut() = Some(config.clone());

        // Close existing windows
        self.app.windows().into_iter().for_each(|w| w.close());

        // Build new UI
        let use_clapper = self.settings_watcher.is_use_clapper();
        let enable_graphics_offload = self.settings_watcher.is_enable_graphics_offload();
        self.app.build_ui(
            &config,
            use_clapper,
            enable_graphics_offload,
            &self.renderers,
            launch_mode,
        );

        // Defer settings application to avoid GStreamer deadlock.
        // build_ui() starts pipeline state transitions via renderer.play(),
        // and setting properties (volume, mute) during the transition blocks
        // the main loop. An idle callback runs after the transition completes.
        let volume = self.settings_watcher.volume();
        let mute = self.settings_watcher.is_mute();
        let fit = self.settings_watcher.content_fit();
        let renderers = self.renderers.clone();
        glib::idle_add_local_once(move || {
            for renderer in renderers.borrow().iter() {
                renderer.set_volume(volume);
                renderer.set_mute(mute);
                renderer.set_content_fit(fit);
            }
        });

        *self.playback_state.borrow_mut() = PlaybackState::Playing;

        // Persist for auto-restore on next daemon startup
        self.settings_watcher.set_last_wallpaper_config(config_json);
        self.settings_watcher.set_last_launch_mode(launch_mode_str);

        Ok(true)
    }

    fn disable_wallpaper(&self) -> bool {
        info!("D-Bus: Disabling wallpaper");

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

    fn pause(&self) -> bool {
        if *self.playback_state.borrow() != PlaybackState::Playing {
            return false;
        }
        info!("D-Bus: Pausing playback");
        for renderer in self.renderers.borrow().iter() {
            renderer.pause();
        }
        *self.playback_state.borrow_mut() = PlaybackState::Paused;
        true
    }

    fn resume(&self) -> bool {
        if *self.playback_state.borrow() != PlaybackState::Paused {
            return false;
        }
        info!("D-Bus: Resuming playback");
        for renderer in self.renderers.borrow().iter() {
            renderer.play();
        }
        *self.playback_state.borrow_mut() = PlaybackState::Playing;
        true
    }

    fn handle_command(&self, cmd: Command) {
        match cmd {
            Command::ApplyWallpaper {
                config_json,
                launch_mode,
                reply,
            } => {
                let result = self.apply_wallpaper(&config_json, &launch_mode);
                let _ = reply.send(result);
            }
            Command::DisableWallpaper { reply } => {
                let _ = reply.send(self.disable_wallpaper());
            }
            Command::Pause { reply } => {
                let _ = reply.send(self.pause());
            }
            Command::Resume { reply } => {
                let _ = reply.send(self.resume());
            }
            Command::Quit => {
                info!("D-Bus: Quitting");
                self.app.quit();
            }
            Command::GetState { reply } => {
                let _ = reply.send(self.playback_state.borrow().as_str().to_string());
            }
        }
    }
}

// --- D-Bus interface (Send + Sync, communicates via channel) ---

struct RendererService {
    cmd_tx: mpsc::SyncSender<Command>,
    /// Connection reference used to emit PropertiesChanged signals.
    conn: std::sync::Mutex<Option<zbus::Connection>>,
}

// SAFETY: RendererService only contains Send + Sync types
unsafe impl Send for RendererService {}
unsafe impl Sync for RendererService {}

#[zbus::interface(name = "io.github.jeffshee.Hotaru.Renderer")]
impl RendererService {
    async fn apply_wallpaper(
        &self,
        config_json: &str,
        launch_mode: &str,
    ) -> zbus::fdo::Result<bool> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.cmd_tx
            .send(Command::ApplyWallpaper {
                config_json: config_json.to_string(),
                launch_mode: launch_mode.to_string(),
                reply: reply_tx,
            })
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel send error: {}", e)))?;

        let result = reply_rx
            .recv()
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel recv error: {}", e)))?
            .map_err(|e| zbus::fdo::Error::Failed(e))?;

        self.emit_state_changed().await;
        Ok(result)
    }

    async fn disable_wallpaper(&self) -> zbus::fdo::Result<bool> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.cmd_tx
            .send(Command::DisableWallpaper { reply: reply_tx })
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel send error: {}", e)))?;

        let result = reply_rx
            .recv()
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel recv error: {}", e)))?;

        self.emit_state_changed().await;
        Ok(result)
    }

    async fn pause(&self) -> zbus::fdo::Result<bool> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.cmd_tx
            .send(Command::Pause { reply: reply_tx })
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel send error: {}", e)))?;

        let result = reply_rx
            .recv()
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel recv error: {}", e)))?;

        self.emit_state_changed().await;
        Ok(result)
    }

    async fn resume(&self) -> zbus::fdo::Result<bool> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.cmd_tx
            .send(Command::Resume { reply: reply_tx })
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel send error: {}", e)))?;

        let result = reply_rx
            .recv()
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel recv error: {}", e)))?;

        self.emit_state_changed().await;
        Ok(result)
    }

    async fn quit(&self) -> zbus::fdo::Result<()> {
        self.cmd_tx
            .send(Command::Quit)
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel send error: {}", e)))
    }

    #[zbus(property)]
    async fn state(&self) -> zbus::fdo::Result<String> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.cmd_tx
            .send(Command::GetState { reply: reply_tx })
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel send error: {}", e)))?;

        reply_rx
            .recv()
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel recv error: {}", e)))
    }
}

impl RendererService {
    /// Emit a PropertiesChanged signal for the State property.
    async fn emit_state_changed(&self) {
        let conn = self.conn.lock().unwrap().clone();
        if let Some(conn) = conn {
            if let Ok(iface_ref) = conn
                .object_server()
                .interface::<_, RendererService>(DBUS_PATH)
                .await
            {
                let ctx = iface_ref.signal_emitter();
                let _ = self.state_changed(ctx).await;
            }
        }
    }
}

// --- Registration ---

/// Register the D-Bus service. Call from the GLib main thread.
/// Sets up a command channel: the D-Bus service sends commands, and
/// the GLib main loop processes them on the main thread.
pub fn register_dbus_service(state: Rc<RendererState>) {
    let (cmd_tx, cmd_rx) = mpsc::sync_channel::<Command>(32);

    // Process commands on the GLib main thread
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        while let Ok(cmd) = cmd_rx.try_recv() {
            state.handle_command(cmd);
        }
        glib::ControlFlow::Continue
    });

    // Spawn the zbus connection on a background thread.
    // zbus uses the async-io/smol ecosystem, so we use async_io::block_on.
    std::thread::spawn(move || {
        async_io::block_on(async move {
            let service = RendererService {
                cmd_tx,
                conn: std::sync::Mutex::new(None),
            };

            let connection = match zbus::connection::Builder::session()
                .expect("Failed to create session builder")
                .name(DBUS_NAME)
                .expect("Failed to set bus name")
                .serve_at(DBUS_PATH, service)
                .expect("Failed to serve at path")
                .build()
                .await
            {
                Ok(conn) => conn,
                Err(e) => {
                    tracing::error!(
                        "Failed to build D-Bus connection: {}. \
                         Is another instance already running?",
                        e
                    );
                    return;
                }
            };

            // Store the connection reference in the service so it can emit signals.
            if let Ok(iface_ref) = connection
                .object_server()
                .interface::<_, RendererService>(DBUS_PATH)
                .await
            {
                let iface = iface_ref.get().await;
                *iface.conn.lock().unwrap() = Some(connection.clone());
            }

            info!("D-Bus service registered: {} at {}", DBUS_NAME, DBUS_PATH);

            // Keep the connection alive
            std::future::pending::<()>().await;
        });
    });
}
