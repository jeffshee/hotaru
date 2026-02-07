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

use std::{env, path::PathBuf};

use clap::{ArgAction, Parser};

use hotaru::prelude::*;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(
        short = 'l',
        long = "launch-mode",
        default_value_t,
        help = "Launch mode"
    )]
    pub launch_mode: LaunchMode,

    #[arg(
        short = 'c',
        long = "config",
        value_name = "FILE",
        help = "Path to the wallpaper config JSON file (not required in daemon mode)"
    )]
    pub config_file: Option<PathBuf>,

    #[arg(
        long,
        default_value = "info",
        help = "Logging level following RUST_LOG format"
    )]
    pub log_level: String,

    #[arg(
        long,
        default_value_t = true,
        action = ArgAction::Set,
        help = "Use Clapper for video playback",
        long_help = "Use Clapper as the video playback backend. \
                     Set to false to use the GTK4 Plugin from GStreamer instead.",
    )]
    use_clapper: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Enable VA decoders for improved performance on Intel/AMD Wayland setups (experimental; default: false)"
    )]
    enable_va: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Enable stateless NVIDIA decoders, which may improve NVIDIA hardware acceleration (experimental; default: false)"
    )]
    enable_nvsl: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Run as a D-Bus daemon, waiting for commands from the frontend"
    )]
    pub daemon: bool,
}

impl Cli {
    pub fn is_use_clapper(&self) -> bool {
        if self.use_clapper {
            true
        } else {
            env::var("USE_CLAPPER").is_ok_and(|v| v == "true")
        }
    }

    pub fn is_enable_va(&self) -> bool {
        if self.enable_va {
            true
        } else {
            env::var("ENABLE_VA").is_ok_and(|v| v == "true")
        }
    }

    pub fn is_enable_nvsl(&self) -> bool {
        if self.enable_nvsl {
            true
        } else {
            env::var("ENABLE_NVSL").is_ok_and(|v| v == "true")
        }
    }
}
