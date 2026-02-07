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

pub mod application;
pub mod constant;
pub mod dbus;
pub mod model;
pub mod monitor_tracker;
pub mod settings_watcher;
pub mod utils;
pub mod widget;
pub mod window;

pub mod prelude {
    pub use crate::application::HotaruApplication;
    pub use crate::constant::*;
    pub use crate::model::{
        LaunchMode, MonitorInfo, MonitorListModelExt, MonitorMap, WallpaperConfig,
    };
    pub use crate::monitor_tracker::MonitorTracker;
    pub use crate::utils::setup_gst;
}
