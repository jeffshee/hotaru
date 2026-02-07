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

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::WallpaperSource;

// TODO: Implementation
#[allow(dead_code)]
type SourceConfigMap = HashMap<WallpaperSource, SourceConfig>;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SourceConfig {
    pub is_mute: bool,
    pub audio_volume: f32,
    pub content_fit: ContentFit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentFit {
    Fill,
    Contain,
    Cover,
    ScaleDown,
}
