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

use std::path::PathBuf;

use clap::Parser;

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
        default_value_t = false,
        help = "Run as a D-Bus daemon, waiting for commands from the frontend"
    )]
    pub daemon: bool,
}
