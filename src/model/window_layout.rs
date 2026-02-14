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

use crate::model::{
    MonitorConfig, MonitorInfo, MonitorMap, WallpaperConfig, WallpaperMode, WallpaperSource,
    WallpaperType,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowLayout {
    pub windows: Vec<WindowInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Viewport {
    pub offset_x: i32,
    pub offset_y: i32,
    pub canvas_width: i32,
    pub canvas_height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WindowInfo {
    Primary {
        monitor: String,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        window_title: String,
        wallpaper_type: WallpaperType,
        wallpaper_source: WallpaperSource,
        viewport: Option<Viewport>,
    },
    Clone {
        monitor: String,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        window_title: String,
        clone_source: String,
        viewport: Option<Viewport>,
    },
}

impl WindowLayout {
    pub fn new(config: &WallpaperConfig, monitor_map: &MonitorMap) -> Self {
        match config.mode {
            WallpaperMode::WallpaperPerMonitor => Self::handle_per_monitor(config, monitor_map),
            WallpaperMode::CloneSingleWallpaper => Self::handle_clone_single(config, monitor_map),
            WallpaperMode::StretchSingleWallpaper => {
                Self::handle_stretch_single(config, monitor_map)
            }
        }
    }

    fn handle_per_monitor(config: &WallpaperConfig, monitor_map: &MonitorMap) -> Self {
        let mut windows = Vec::new();

        for monitor in &config.monitors {
            if let MonitorConfig::Primary {
                monitor,
                wallpaper_type,
                wallpaper_source,
            } = monitor
            {
                if let Some(MonitorInfo {
                    x,
                    y,
                    width,
                    height,
                }) = monitor_map.get(monitor)
                {
                    windows.push(WindowInfo::Primary {
                        monitor: monitor.clone(),
                        window_x: *x,
                        window_y: *y,
                        window_width: *width,
                        window_height: *height,
                        window_title: format!("Live Wallpaper - {monitor}"),
                        wallpaper_type: *wallpaper_type,
                        wallpaper_source: wallpaper_source.clone(),
                        viewport: None,
                    })
                }
            }
        }

        Self { windows }
    }

    fn handle_clone_single(config: &WallpaperConfig, monitor_map: &MonitorMap) -> Self {
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
            if let Some(MonitorInfo {
                x,
                y,
                width,
                height,
            }) = monitor_map.get(monitor)
            {
                primary = Some(monitor.clone());

                windows.push(WindowInfo::Primary {
                    monitor: monitor.clone(),
                    window_x: *x,
                    window_y: *y,
                    window_width: *width,
                    window_height: *height,
                    window_title: format!("Live Wallpaper - {monitor}"),
                    wallpaper_type: *wallpaper_type,
                    wallpaper_source: wallpaper_source.clone(),
                    viewport: None,
                })
            }
        }

        // Add the clones
        if let Some(primary_monitor) = primary {
            for monitor in &config.monitors {
                if let MonitorConfig::Clone { monitor, .. } = monitor {
                    if let Some(MonitorInfo {
                        x,
                        y,
                        width,
                        height,
                    }) = monitor_map.get(monitor)
                    {
                        windows.push(WindowInfo::Clone {
                            monitor: monitor.clone(),
                            window_x: *x,
                            window_y: *y,
                            window_width: *width,
                            window_height: *height,
                            window_title: format!(
                                "Live Wallpaper - {monitor} (Clone of {primary_monitor})"
                            ),
                            clone_source: primary_monitor.clone(),
                            viewport: None,
                        })
                    }
                }
            }
        }

        Self { windows }
    }

    fn handle_stretch_single(config: &WallpaperConfig, monitor_map: &MonitorMap) -> Self {
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

        for (i, (monitor_name, info)) in monitors.iter().enumerate() {
            let viewport = Some(Viewport {
                offset_x: info.x,
                offset_y: info.y,
                canvas_width: box_width,
                canvas_height: box_height,
            });

            if i == 0 {
                primary_name = Some(monitor_name.to_string());
                windows.push(WindowInfo::Primary {
                    monitor: monitor_name.to_string(),
                    window_x: info.x,
                    window_y: info.y,
                    window_width: info.width,
                    window_height: info.height,
                    window_title: format!("Live Wallpaper - {} (Stretch)", monitor_name),
                    wallpaper_type,
                    wallpaper_source: wallpaper_source.clone(),
                    viewport,
                });
            } else {
                windows.push(WindowInfo::Clone {
                    monitor: monitor_name.to_string(),
                    window_x: info.x,
                    window_y: info.y,
                    window_width: info.width,
                    window_height: info.height,
                    window_title: format!("Live Wallpaper - {} (Stretch)", monitor_name),
                    clone_source: primary_name.as_ref().unwrap().clone(),
                    viewport,
                });
            }
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
            match window {
                WindowInfo::Primary {
                    monitor,
                    window_x,
                    window_y,
                    window_width,
                    window_height,
                    ..
                } => {
                    if monitor == "DP-1" {
                        assert_eq!(*window_x, 0);
                        assert_eq!(*window_y, 0);
                        assert_eq!(*window_width, 1920);
                        assert_eq!(*window_height, 1080);
                        dp1_found = true;
                    }
                    if monitor == "DP-2" {
                        assert_eq!(*window_x, 1920);
                        assert_eq!(*window_y, 0);
                        assert_eq!(*window_width, 2560);
                        assert_eq!(*window_height, 1440);
                        dp2_found = true;
                    }
                }
                _ => panic!("Unexpected clone window in per-monitor mode"),
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
            match window {
                WindowInfo::Primary { monitor, .. } => {
                    assert_eq!(monitor, "DP-1");
                    primary_found = true;
                }
                WindowInfo::Clone {
                    monitor,
                    clone_source,
                    ..
                } => {
                    assert_eq!(monitor, "DP-2");
                    assert_eq!(clone_source, "DP-1");
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
        if let WindowInfo::Primary {
            monitor,
            window_x,
            window_y,
            window_width,
            window_height,
            viewport,
            ..
        } = &layout.windows[0]
        {
            assert_eq!(monitor, "DP-1");
            assert_eq!(*window_x, 1920);
            assert_eq!(*window_y, 600);
            assert_eq!(*window_width, 2560);
            assert_eq!(*window_height, 1440);
            let vp = viewport.as_ref().unwrap();
            assert_eq!(vp.offset_x, 1920);
            assert_eq!(vp.offset_y, 600);
            assert_eq!(vp.canvas_width, expected_canvas.0);
            assert_eq!(vp.canvas_height, expected_canvas.1);
        } else {
            panic!("Expected primary window for first monitor");
        }

        // Remaining monitors are Clones
        for window in &layout.windows[1..] {
            if let WindowInfo::Clone {
                clone_source,
                viewport,
                ..
            } = window
            {
                assert_eq!(clone_source, "DP-1");
                let vp = viewport.as_ref().unwrap();
                assert_eq!(vp.canvas_width, expected_canvas.0);
                assert_eq!(vp.canvas_height, expected_canvas.1);
            } else {
                panic!("Expected clone window for non-primary monitor");
            }
        }
    }
}
