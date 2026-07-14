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

use std::rc::Rc;

use gtk::glib;
use tracing::info;

use crate::state::RendererState;

pub const DBUS_NAME: &str = "io.github.jeffshee.Hotaru";
pub const DBUS_PATH: &str = "/io/github/jeffshee/Hotaru";

// --- Commands sent from the D-Bus thread to the GLib main thread ---

enum Command {
    ApplyWallpaper {
        config_json: String,
        launch_mode: String,
        reply: async_channel::Sender<Result<bool, String>>,
    },
    DisableWallpaper {
        reply: async_channel::Sender<bool>,
    },
    Pause {
        reply: async_channel::Sender<bool>,
    },
    Resume {
        reply: async_channel::Sender<bool>,
    },
    Quit,
    GetState {
        reply: async_channel::Sender<String>,
    },
}

fn handle_command(state: &RendererState, cmd: Command) {
    // Replies use send_blocking: each reply channel has capacity 1 and a
    // single send, so this never actually blocks the main thread.
    match cmd {
        Command::ApplyWallpaper {
            config_json,
            launch_mode,
            reply,
        } => {
            let result = state.apply_wallpaper(&config_json, &launch_mode);
            let _ = reply.send_blocking(result);
        }
        Command::DisableWallpaper { reply } => {
            let _ = reply.send_blocking(state.disable_wallpaper());
        }
        Command::Pause { reply } => {
            let _ = reply.send_blocking(state.pause());
        }
        Command::Resume { reply } => {
            let _ = reply.send_blocking(state.resume());
        }
        Command::Quit => {
            state.quit();
        }
        Command::GetState { reply } => {
            let _ = reply.send_blocking(state.playback_state.borrow().as_str().to_string());
        }
    }
}

// --- D-Bus interface (Send + Sync, communicates via channel) ---

struct RendererService {
    cmd_tx: async_channel::Sender<Command>,
    /// Connection reference used to emit PropertiesChanged signals.
    conn: std::sync::Mutex<Option<zbus::Connection>>,
}

impl RendererService {
    /// Send a command to the main thread and await its reply.
    async fn request<R>(
        &self,
        make_command: impl FnOnce(async_channel::Sender<R>) -> Command,
    ) -> zbus::fdo::Result<R> {
        let (reply_tx, reply_rx) = async_channel::bounded(1);
        self.cmd_tx
            .send(make_command(reply_tx))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel send error: {}", e)))?;
        reply_rx
            .recv()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel recv error: {}", e)))
    }
}

#[zbus::interface(name = "io.github.jeffshee.Hotaru.Renderer")]
impl RendererService {
    async fn apply_wallpaper(
        &self,
        config_json: &str,
        launch_mode: &str,
    ) -> zbus::fdo::Result<bool> {
        let config_json = config_json.to_string();
        let launch_mode = launch_mode.to_string();
        let result = self
            .request(|reply| Command::ApplyWallpaper {
                config_json,
                launch_mode,
                reply,
            })
            .await?
            .map_err(zbus::fdo::Error::Failed)?;

        self.emit_state_changed().await;
        Ok(result)
    }

    async fn disable_wallpaper(&self) -> zbus::fdo::Result<bool> {
        let result = self
            .request(|reply| Command::DisableWallpaper { reply })
            .await?;
        self.emit_state_changed().await;
        Ok(result)
    }

    async fn pause(&self) -> zbus::fdo::Result<bool> {
        let result = self.request(|reply| Command::Pause { reply }).await?;
        self.emit_state_changed().await;
        Ok(result)
    }

    async fn resume(&self) -> zbus::fdo::Result<bool> {
        let result = self.request(|reply| Command::Resume { reply }).await?;
        self.emit_state_changed().await;
        Ok(result)
    }

    async fn quit(&self) -> zbus::fdo::Result<()> {
        self.cmd_tx
            .send(Command::Quit)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(format!("Channel send error: {}", e)))
    }

    #[zbus(property)]
    async fn state(&self) -> zbus::fdo::Result<String> {
        self.request(|reply| Command::GetState { reply }).await
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
    let (cmd_tx, cmd_rx) = async_channel::bounded::<Command>(32);

    // Process commands on the GLib main thread as they arrive (the
    // channel integrates with the main loop; no polling).
    glib::spawn_future_local(async move {
        while let Ok(cmd) = cmd_rx.recv().await {
            handle_command(&state, cmd);
        }
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
