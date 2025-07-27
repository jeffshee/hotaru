use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::model::{MonitorInfo, MonitorMap};

// Live Wallpaper Config

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiveWallpaperConfig {
    pub mode: WallpaperMode,
    pub monitors: Vec<MonitorConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperMode {
    WallpaperPerMonitor,
    CloneSingleWallpaper,
    StretchSingleWallpaper,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperSource {
    Filepath { filepath: String },
    Uri { uri: String },
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperType {
    Video,
    Web,
}

// Window Layout

#[derive(Serialize, Deserialize, Debug)]
pub struct WindowLayout {
    pub windows: Vec<WindowInfo>,
}

#[derive(Serialize, Deserialize, Debug)]
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
    },
    Clone {
        monitor: String,
        window_x: i32,
        window_y: i32,
        window_width: i32,
        window_height: i32,
        window_title: String,
        clone_source: String,
    },
}

// TODO: Implementation
// SourceConfig keyed by wallpaper_source
#[allow(dead_code)]
type SourceConfigMap = HashMap<String, SourceConfig>;

#[derive(Serialize, Deserialize, Debug)]
pub struct SourceConfig {
    pub is_mute: bool,
    pub audio_volume: f32,
    pub content_fit: ContentFit,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ContentFit {
    Fill,
    Contain,
    Cover,
    ScaleDown,
}

// Conversion

pub fn convert_to_window_layout(
    config: &LiveWallpaperConfig,
    monitor_map: &MonitorMap,
) -> WindowLayout {
    match config.mode {
        WallpaperMode::WallpaperPerMonitor => handle_per_monitor(config, monitor_map),
        WallpaperMode::CloneSingleWallpaper => handle_clone(config, monitor_map),
        WallpaperMode::StretchSingleWallpaper => handle_stretch(config, monitor_map),
    }
}

fn handle_per_monitor(config: &LiveWallpaperConfig, monitor_map: &MonitorMap) -> WindowLayout {
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
                })
            }
        }
    }

    WindowLayout { windows }
}

fn handle_clone(config: &LiveWallpaperConfig, monitor_map: &MonitorMap) -> WindowLayout {
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
                    })
                }
            }
        }
    }

    WindowLayout { windows }
}

fn handle_stretch(config: &LiveWallpaperConfig, monitor_map: &MonitorMap) -> WindowLayout {
    let mut windows = Vec::new();

    // Calculate bounding box, given the leftmost rect has x == 0 and the topmost rect has y == 0
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

    if let Some(MonitorConfig::Primary {
        wallpaper_type,
        wallpaper_source,
        ..
    }) = config.monitors.first()
    {
        windows.push(WindowInfo::Primary {
            monitor: "STRETCH".to_string(),
            window_x: 0,
            window_y: 0,
            window_width: box_width,
            window_height: box_height,
            window_title: "Live Wallpaper - STRETCH".to_string(),
            wallpaper_type: *wallpaper_type,
            wallpaper_source: wallpaper_source.clone(),
        });
    }

    WindowLayout { windows }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wallpaper_per_monitor() {
        let config = LiveWallpaperConfig {
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

        let layout = convert_to_window_layout(&config, &monitor_map);

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
        let config = LiveWallpaperConfig {
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

        let layout = convert_to_window_layout(&config, &monitor_map);

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
        let config = LiveWallpaperConfig {
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

        let layout = convert_to_window_layout(&config, &monitor_map);

        assert_eq!(layout.windows.len(), 1);

        if let WindowInfo::Primary {
            monitor,
            window_x,
            window_y,
            window_width,
            window_height,
            ..
        } = &layout.windows[0]
        {
            assert_eq!(monitor, "STRETCH");
            assert_eq!(*window_x, 0);
            assert_eq!(*window_y, 0);
            assert_eq!(*window_width, 5920);
            assert_eq!(*window_height, 2680);
        } else {
            panic!("Expected primary window for stretch mode");
        }
    }
}
