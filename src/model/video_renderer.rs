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

use strum_macros::{Display, EnumString};

/// Backend used to render video wallpapers.
///
/// The string representation matches the `video-renderer` GSettings key
/// ("mpv", "gst-gtk4"). mpv is the default for its performance (notably
/// working hardware decoding); GstGtk4 is the fallback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumString, Display)]
#[strum(serialize_all = "kebab_case")]
pub enum VideoRenderer {
    #[default]
    Mpv,
    GstGtk4,
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr as _;

    #[test]
    fn test_video_renderer_default() {
        assert_eq!(VideoRenderer::default(), VideoRenderer::Mpv);
    }

    #[test]
    fn test_video_renderer_from_str() {
        assert_eq!(
            VideoRenderer::from_str("gst-gtk4").unwrap(),
            VideoRenderer::GstGtk4
        );
        assert_eq!(VideoRenderer::from_str("mpv").unwrap(), VideoRenderer::Mpv);
        // Unknown values must not parse; SettingsWatcher falls back to the
        // default in that case.
        assert!(VideoRenderer::from_str("unknown").is_err());
    }

    #[test]
    fn test_video_renderer_to_string() {
        assert_eq!(VideoRenderer::GstGtk4.to_string(), "gst-gtk4");
        assert_eq!(VideoRenderer::Mpv.to_string(), "mpv");
    }
}
