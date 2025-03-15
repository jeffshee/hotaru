use crate::monitor::{MonitorError, MonitorExt, MonitorManager};
use gtk::gdk::{prelude::MonitorExt as _, Monitor};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LayoutError {
    #[error(transparent)]
    MonitorError(#[from] MonitorError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceIdentifier {
    Filepath { filepath: String },
    Uri { uri: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceType {
    Video,
    Web,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    #[serde(flatten)]
    pub identifier: SourceIdentifier,
    pub r#type: SourceType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultLayout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_monitor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MonitorConfig {
    Source {
        monitor: String,
        #[serde(flatten)]
        source: SourceIdentifier,
        r#type: SourceType,
    },
    Mirror {
        monitor: String,
        mirror_of: String,
    },
}

impl MonitorConfig {
    pub fn is_valid(&self, monitors: &[Monitor]) -> bool {
        match self {
            MonitorConfig::Source {
                monitor,
                source: _,
                r#type: _,
            } => monitors.iter().any(|m| m.is_connector(monitor)),
            MonitorConfig::Mirror { monitor, mirror_of } => {
                monitor != mirror_of
                    && monitors.iter().any(|m| m.is_connector(monitor))
                    && monitors.iter().any(|m| m.is_connector(mirror_of))
            }
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FinalLayout {
    pub configs: Vec<MonitorConfig>,
}

pub type CustomLayout = FinalLayout;

#[derive(Debug, Clone, PartialEq)]
pub enum Layout {
    Default(DefaultLayout),
    Custom(CustomLayout),
}

impl Layout {
    pub fn finalize(&self, monitors: &[Monitor]) -> Result<FinalLayout, LayoutError> {
        match self {
            Layout::Default(layout) => {
                // Validate primary monitor
                let primary_monitor = layout
                    .primary_monitor
                    .as_ref()
                    .filter(|name| monitors.iter().any(|monitor| monitor.is_connector(name)))
                    .map(|s| s.to_owned())
                    .or_else(MonitorManager::primary_monitor_connector);

                let primary_monitor = match primary_monitor {
                    Some(monitor) => monitor,
                    None => return Ok(FinalLayout::default()),
                };

                let mut configs = Vec::new();

                // Add source config
                match layout.source.as_ref() {
                    Some(source) => {
                        configs.push(MonitorConfig::Source {
                            monitor: primary_monitor.clone(),
                            source: source.identifier.clone(),
                            r#type: source.r#type.clone(),
                        });
                    }
                    None => return Ok(FinalLayout::default()),
                }

                // Add mirror configs
                let mirror_configs = monitors
                    .iter()
                    .filter(|monitor| {
                        !monitor.is_connector(&primary_monitor) && monitor.connector().is_some()
                    })
                    .map(|monitor| MonitorConfig::Mirror {
                        monitor: monitor.connector().unwrap().to_string(), // it's safe to unwrap here
                        mirror_of: primary_monitor.clone(),
                    });

                configs.extend(mirror_configs);

                Ok(FinalLayout { configs })
            }
            Layout::Custom(layout) => {
                let configs = layout
                    .configs
                    .iter()
                    .filter(|cfg| cfg.is_valid(&monitors))
                    .cloned()
                    .collect();
                Ok(FinalLayout { configs })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::MonitorListModelExt;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn init() {
        INIT.call_once(|| {
            gtk::init().unwrap();
        });
    }

    #[test]
    fn test_default_layout_to_json() {
        let default_layout = DefaultLayout {
            source: Some(Source {
                identifier: SourceIdentifier::Filepath {
                    filepath: "./test.webm".to_string(),
                },
                r#type: SourceType::Video,
            }),
            primary_monitor: None,
        };

        let serialized = serde_json::to_string_pretty(&default_layout).unwrap();
        println!("{}", serialized);

        let json = r#"{
            "source": {
                "filepath": "./test.webm",
                "type": "video"
            }
        }"#;

        let deserialized: DefaultLayout = serde_json::from_str(json).unwrap();
        assert_eq!(default_layout, deserialized);
    }

    #[test]
    fn test_finalize_default_layout() {
        init();
        let monitors = MonitorManager::monitors()
            .unwrap()
            .try_to_monitor_vec()
            .unwrap();

        let json = r#"{
            "source": {
                "filepath": "./test.webm",
                "type": "video"
            }
        }"#;

        let default_layout: DefaultLayout = serde_json::from_str(json).unwrap();
        let layout = Layout::Default(default_layout);
        let final_layout = layout.finalize(&monitors).unwrap();

        println!("{:#?}", final_layout);
        assert_ne!(final_layout.configs.len(), 0);
    }

    #[test]
    fn test_finalize_default_layout_with_primary_monitor() {
        init();
        let monitors = MonitorManager::monitors()
            .unwrap()
            .try_to_monitor_vec()
            .unwrap();

        let json = r#"{
            "source": {
                "uri": "https://jeffshee.github.io/herta-wallpaper/",
                "type": "web"
            },
            "primary_monitor": "eDP-1"
        }"#;

        let default_layout: DefaultLayout = serde_json::from_str(json).unwrap();
        let layout = Layout::Default(default_layout);
        let final_layout = layout.finalize(&monitors).unwrap();

        println!("{:#?}", final_layout);
        assert_ne!(final_layout.configs.len(), 0);
    }

    #[test]
    fn test_finalize_default_layout_empty() {
        init();
        let monitors = MonitorManager::monitors()
            .unwrap()
            .try_to_monitor_vec()
            .unwrap();

        let json = r#"{}"#;
        let default_layout: DefaultLayout = serde_json::from_str(json).unwrap();
        let layout = Layout::Default(default_layout);
        let final_layout = layout.finalize(&monitors).unwrap();

        println!("{:#?}", final_layout);
        assert_eq!(final_layout, FinalLayout::default());
    }

    #[test]
    fn test_custom_layout_to_json() {
        let configs = vec![
            MonitorConfig::Source {
                monitor: "DP-4".to_string(),
                source: SourceIdentifier::Filepath {
                    filepath: "./test.webm".to_string(),
                },
                r#type: SourceType::Video,
            },
            MonitorConfig::Mirror {
                monitor: "DP-2".to_string(),
                mirror_of: "DP-4".to_string(),
            },
        ];
        let custom_layout = CustomLayout { configs };

        let serialized = serde_json::to_string_pretty(&custom_layout).unwrap();
        println!("{}", serialized);

        let json = r#"{
            "configs": [
                {
                "monitor": "DP-4",
                "filepath": "./test.webm",
                "type": "video"
                },
                {
                "monitor": "DP-2",
                "mirror_of": "DP-4"
                }
            ]
        }"#;

        let deserialized: CustomLayout = serde_json::from_str(json).unwrap();
        assert_eq!(custom_layout, deserialized);
    }

    #[test]
    fn test_finalize_custom_layout() {
        init();
        let monitors = MonitorManager::monitors()
            .unwrap()
            .try_to_monitor_vec()
            .unwrap();

        let json = r#"{
            "configs": [
                {
                    "monitor": "DP-4",
                    "filepath": "./test.webm",
                    "type": "video"
                },
                {
                    "monitor": "DP-2",
                    "mirror_of": "DP-4"
                }
            ]
        }"#;
        let custom_layout: CustomLayout = serde_json::from_str(json).unwrap();
        let custom_layout = Layout::Custom(custom_layout);
        let final_layout = custom_layout.finalize(&monitors).unwrap();

        println!("{:#?}", final_layout);
    }

    #[test]
    fn test_finalize_custom_layout_multi() {
        init();
        let monitors = MonitorManager::monitors()
            .unwrap()
            .try_to_monitor_vec()
            .unwrap();

        let json = r#"{
            "configs": [
                {
                    "monitor": "DP-4",
                    "filepath": "./test.webm",
                    "type": "video"
                },
                {
                    "monitor": "DP-2",
                    "uri": "https://jeffshee.github.io/herta-wallpaper/",
                    "type": "web"
                }
            ]
        }"#;
        let custom_layout: CustomLayout = serde_json::from_str(json).unwrap();
        let custom_layout = Layout::Custom(custom_layout);

        let final_layout = custom_layout.finalize(&monitors).unwrap();
        println!("{:#?}", final_layout);
    }

    #[test]
    fn test_finalize_custom_layout_empty() {
        init();
        let monitors = MonitorManager::monitors()
            .unwrap()
            .try_to_monitor_vec()
            .unwrap();

        let json = r#"{
            "configs": []
        }"#;
        let custom_layout: CustomLayout = serde_json::from_str(json).unwrap();
        let custom_layout = Layout::Custom(custom_layout);
        let final_layout = custom_layout.finalize(&monitors).unwrap();

        println!("{:#?}", final_layout);
        assert_eq!(final_layout, FinalLayout::default());
    }
}
