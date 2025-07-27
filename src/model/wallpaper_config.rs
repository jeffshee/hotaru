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
