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
