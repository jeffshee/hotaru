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

use crate::model::{
    MonitorConfig, MonitorInfo, MonitorMap, WallpaperConfig, WallpaperMode, WallpaperSource,
    WallpaperType,
};

/// The set of windows to create for a wallpaper config on the current
/// monitors. Windows are ordered primaries-first, so a consumer building
/// them in order always has a clone's source renderer available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowLayout {
    pub windows: Vec<WindowInfo>,
}

/// Window position and size, in the compositor's global coordinate space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowGeometry {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<MonitorInfo> for WindowGeometry {
    fn from(info: MonitorInfo) -> Self {
        Self {
            x: info.x,
            y: info.y,
            width: info.width,
            height: info.height,
        }
    }
}

/// The visible region of an oversized canvas (stretch mode): the child is
/// allocated at canvas size and shifted by the offset, clipped to the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Viewport {
    pub offset_x: i32,
    pub offset_y: i32,
    pub canvas_width: i32,
    pub canvas_height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    /// Connector name of the monitor this window covers (e.g. "DP-1").
    pub monitor: String,
    pub geometry: WindowGeometry,
    pub title: String,
    pub viewport: Option<Viewport>,
    pub role: WindowRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowRole {
    /// Runs a renderer for the wallpaper source.
    Primary {
        wallpaper_type: WallpaperType,
        wallpaper_source: WallpaperSource,
    },
    /// Mirrors the primary renderer of another monitor.
    Clone {
        /// Monitor (connector name) whose primary renderer to mirror.
        source: String,
    },
}

impl WindowLayout {
    pub fn new(config: &WallpaperConfig, monitor_map: &MonitorMap) -> Self {
        match config.mode {
            WallpaperMode::WallpaperPerMonitor => Self::layout_per_monitor(config, monitor_map),
            WallpaperMode::CloneSingleWallpaper => Self::layout_clone_single(config, monitor_map),
            WallpaperMode::StretchSingleWallpaper => {
                Self::layout_stretch_single(config, monitor_map)
            }
        }
    }

    fn layout_per_monitor(config: &WallpaperConfig, monitor_map: &MonitorMap) -> Self {
        let mut windows = Vec::new();

        for monitor_config in &config.monitors {
            if let MonitorConfig::Primary {
                monitor,
                wallpaper_type,
                wallpaper_source,
            } = monitor_config
            {
                if let Some(info) = monitor_map.get(monitor) {
                    windows.push(WindowInfo {
                        monitor: monitor.clone(),
                        geometry: (*info).into(),
                        title: format!("Live Wallpaper - {monitor}"),
                        viewport: None,
                        role: WindowRole::Primary {
                            wallpaper_type: *wallpaper_type,
                            wallpaper_source: wallpaper_source.clone(),
                        },
                    })
                }
            }
        }

        Self { windows }
    }

    fn layout_clone_single(config: &WallpaperConfig, monitor_map: &MonitorMap) -> Self {
        let mut windows = Vec::new();
        let mut primary = None;

        // Add the primary
        if let Some(MonitorConfig::Primary {
            monitor,
            wallpaper_type,
            wallpaper_source,
        }) = config
            .monitors
            .iter()
            .find(|m| matches!(m, MonitorConfig::Primary { .. }))
        {
            if let Some(info) = monitor_map.get(monitor) {
                primary = Some(monitor.clone());

                windows.push(WindowInfo {
                    monitor: monitor.clone(),
                    geometry: (*info).into(),
                    title: format!("Live Wallpaper - {monitor}"),
                    viewport: None,
                    role: WindowRole::Primary {
                        wallpaper_type: *wallpaper_type,
                        wallpaper_source: wallpaper_source.clone(),
                    },
                })
            }
        }

        // Add the clones
        if let Some(primary_monitor) = primary {
            for monitor_config in &config.monitors {
                if let MonitorConfig::Clone { monitor, .. } = monitor_config {
                    if let Some(info) = monitor_map.get(monitor) {
                        windows.push(WindowInfo {
                            monitor: monitor.clone(),
                            geometry: (*info).into(),
                            title: format!(
                                "Live Wallpaper - {monitor} (Clone of {primary_monitor})"
                            ),
                            viewport: None,
                            role: WindowRole::Clone {
                                source: primary_monitor.clone(),
                            },
                        })
                    }
                }
            }
        }

        Self { windows }
    }

    fn layout_stretch_single(config: &WallpaperConfig, monitor_map: &MonitorMap) -> Self {
        let mut windows = Vec::new();

        // Calculate bounding box of all monitors
        let (box_width, box_height) = monitor_map.values().fold(
            (0, 0),
            |acc,
             MonitorInfo {
                 x,
                 y,
                 width,
                 height,
             }| { (acc.0.max(*x + *width), acc.1.max(*y + *height)) },
        );

        let (wallpaper_type, wallpaper_source) = match config.monitors.first() {
            Some(MonitorConfig::Primary {
                wallpaper_type,
                wallpaper_source,
                ..
            }) => (*wallpaper_type, wallpaper_source.clone()),
            _ => return Self { windows },
        };

        // Create one window per monitor. The first becomes Primary (renders
        // the video), the rest become Clones (mirror the paintable). Each
        // window carries a Viewport describing its offset within the canvas.
        let mut monitors: Vec<_> = monitor_map.iter().collect();
        monitors.sort_by_key(|(name, _)| (*name).clone());

        let mut primary_name = None;

        for (monitor_name, info) in monitors {
            let viewport = Some(Viewport {
                offset_x: info.x,
                offset_y: info.y,
                canvas_width: box_width,
                canvas_height: box_height,
            });

            let role = match &primary_name {
                None => {
                    primary_name = Some(monitor_name.clone());
                    WindowRole::Primary {
                        wallpaper_type,
                        wallpaper_source: wallpaper_source.clone(),
                    }
                }
                Some(primary) => WindowRole::Clone {
                    source: primary.clone(),
                },
            };

            windows.push(WindowInfo {
                monitor: monitor_name.clone(),
                geometry: (*info).into(),
                title: format!("Live Wallpaper - {} (Stretch)", monitor_name),
                viewport,
                role,
            });
        }

        Self { windows }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    #[test]
    fn test_wallpaper_per_monitor() {
        let config = WallpaperConfig {
            mode: WallpaperMode::WallpaperPerMonitor,
            monitors: vec![
                MonitorConfig::Primary {
                    monitor: "DP-1".into(),
                    wallpaper_type: WallpaperType::Video,
                    wallpaper_source: WallpaperSource::Filepath {
                        filepath: "/videos/test.mp4".into(),
                    },
                },
                MonitorConfig::Primary {
                    monitor: "DP-2".into(),
                    wallpaper_type: WallpaperType::Web,
                    wallpaper_source: WallpaperSource::Uri {
                        uri: "https://example.com".into(),
                    },
                },
            ],
        };

        let monitor_map = HashMap::from([
            (
                "DP-1".to_string(),
                MonitorInfo {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
            ),
            (
                "DP-2".to_string(),
                MonitorInfo {
                    x: 1920,
                    y: 0,
                    width: 2560,
                    height: 1440,
                },
            ),
        ]);

        let layout = WindowLayout::new(&config, &monitor_map);

        assert_eq!(layout.windows.len(), 2);

        let mut dp1_found = false;
        let mut dp2_found = false;

        for window in &layout.windows {
            assert!(
                matches!(window.role, WindowRole::Primary { .. }),
                "Unexpected clone window in per-monitor mode"
            );
            if window.monitor == "DP-1" {
                assert_eq!(
                    window.geometry,
                    WindowGeometry {
                        x: 0,
                        y: 0,
                        width: 1920,
                        height: 1080
                    }
                );
                dp1_found = true;
            }
            if window.monitor == "DP-2" {
                assert_eq!(
                    window.geometry,
                    WindowGeometry {
                        x: 1920,
                        y: 0,
                        width: 2560,
                        height: 1440
                    }
                );
                dp2_found = true;
            }
        }

        assert!(dp1_found && dp2_found);
    }

    #[test]
    fn test_clone_single_wallpaper() {
        let config = WallpaperConfig {
            mode: WallpaperMode::CloneSingleWallpaper,
            monitors: vec![
                MonitorConfig::Primary {
                    monitor: "DP-1".into(),
                    wallpaper_type: WallpaperType::Video,
                    wallpaper_source: WallpaperSource::Filepath {
                        filepath: "/videos/main.mp4".into(),
                    },
                },
                MonitorConfig::Clone {
                    monitor: "DP-2".into(),
                    clone_source: None,
                },
            ],
        };

        let monitor_map = HashMap::from([
            (
                "DP-1".to_string(),
                MonitorInfo {
                    x: 0,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
            ),
            (
                "DP-2".to_string(),
                MonitorInfo {
                    x: 1920,
                    y: 0,
                    width: 1920,
                    height: 1080,
                },
            ),
        ]);

        let layout = WindowLayout::new(&config, &monitor_map);

        assert_eq!(layout.windows.len(), 2);

        let mut primary_found = false;
        let mut clone_found = false;

        for window in &layout.windows {
            match &window.role {
                WindowRole::Primary { .. } => {
                    assert_eq!(window.monitor, "DP-1");
                    primary_found = true;
                }
                WindowRole::Clone { source } => {
                    assert_eq!(window.monitor, "DP-2");
                    assert_eq!(source, "DP-1");
                    clone_found = true;
                }
            }
        }

        assert!(primary_found && clone_found);
    }

    #[test]
    fn test_stretch_single_wallpaper() {
        let config = WallpaperConfig {
            mode: WallpaperMode::StretchSingleWallpaper,
            monitors: vec![MonitorConfig::Primary {
                monitor: "STRETCH".into(),
                wallpaper_type: WallpaperType::Video,
                wallpaper_source: WallpaperSource::Filepath {
                    filepath: "/videos/wide.mp4".into(),
                },
            }],
        };

        let monitor_map = HashMap::from([
            (
                "eDP-1".to_string(),
                MonitorInfo {
                    x: 0,
                    y: 1600,
                    width: 1920,
                    height: 1080,
                },
            ),
            (
                "DP-1".to_string(),
                MonitorInfo {
                    x: 1920,
                    y: 600,
                    width: 2560,
                    height: 1440,
                },
            ),
            (
                "DP-2".to_string(),
                MonitorInfo {
                    x: 4480,
                    y: 0,
                    width: 1440,
                    height: 2560,
                },
            ),
        ]);

        let layout = WindowLayout::new(&config, &monitor_map);

        // One window per monitor (sorted: DP-1, DP-2, eDP-1)
        assert_eq!(layout.windows.len(), 3);

        // Bounding box: 5920 x 2680
        let expected_canvas = (5920, 2680);

        // First monitor (DP-1) is Primary
        let first = &layout.windows[0];
        assert!(matches!(first.role, WindowRole::Primary { .. }));
        assert_eq!(first.monitor, "DP-1");
        assert_eq!(
            first.geometry,
            WindowGeometry {
                x: 1920,
                y: 600,
                width: 2560,
                height: 1440
            }
        );
        let vp = first.viewport.as_ref().unwrap();
        assert_eq!(vp.offset_x, 1920);
        assert_eq!(vp.offset_y, 600);
        assert_eq!(vp.canvas_width, expected_canvas.0);
        assert_eq!(vp.canvas_height, expected_canvas.1);

        // Remaining monitors are Clones
        for window in &layout.windows[1..] {
            let WindowRole::Clone { source } = &window.role else {
                panic!("Expected clone window for non-primary monitor");
            };
            assert_eq!(source, "DP-1");
            let vp = window.viewport.as_ref().unwrap();
            assert_eq!(vp.canvas_width, expected_canvas.0);
            assert_eq!(vp.canvas_height, expected_canvas.1);
        }
    }
}
