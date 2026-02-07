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

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WallpaperConfig {
    pub mode: WallpaperMode,
    pub monitors: Vec<MonitorConfig>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperMode {
    WallpaperPerMonitor,
    CloneSingleWallpaper,
    StretchSingleWallpaper,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MonitorConfig {
    Primary {
        monitor: String,
        wallpaper_type: WallpaperType,
        #[serde(flatten)]
        wallpaper_source: WallpaperSource,
    },
    Clone {
        monitor: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        clone_source: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperSource {
    Filepath { filepath: String },
    Uri { uri: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperType {
    Video,
    Web,
}
