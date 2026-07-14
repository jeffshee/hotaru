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

use gtk::glib;
use strum_macros::{Display, EnumString};

/// How wallpaper windows integrate with the desktop.
///
/// The string representation (kebab-case, e.g. "x11-desktop") is the CLI /
/// D-Bus wire format. As a `glib::Boxed` type it can also be carried in
/// GObject properties directly (see `HotaruApplicationWindow`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumString, Display, glib::Boxed)]
#[boxed_type(name = "HotaruLaunchMode")]
#[strum(serialize_all = "kebab_case")]
pub enum LaunchMode {
    #[default]
    X11Desktop,
    WaylandLayerShell,
    GnomeExtHanabi,
    Windowed,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr as _;

    #[test]
    fn test_launch_mode_default() {
        let default_mode = LaunchMode::default();
        assert_eq!(default_mode, LaunchMode::X11Desktop);
    }

    #[test]
    fn test_launch_mode_from_str() {
        assert_eq!(
            LaunchMode::from_str("x11-desktop").unwrap(),
            LaunchMode::X11Desktop
        );
        assert_eq!(
            LaunchMode::from_str("wayland-layer-shell").unwrap(),
            LaunchMode::WaylandLayerShell
        );
        assert_eq!(
            LaunchMode::from_str("gnome-ext-hanabi").unwrap(),
            LaunchMode::GnomeExtHanabi
        );
        assert_eq!(LaunchMode::from_str("windowed").unwrap(), LaunchMode::Windowed);
    }

    #[test]
    fn test_launch_mode_to_string() {
        assert_eq!(LaunchMode::X11Desktop.to_string(), "x11-desktop");
        assert_eq!(
            LaunchMode::WaylandLayerShell.to_string(),
            "wayland-layer-shell"
        );
        assert_eq!(LaunchMode::GnomeExtHanabi.to_string(), "gnome-ext-hanabi");
        assert_eq!(LaunchMode::Windowed.to_string(), "windowed");
    }
}
