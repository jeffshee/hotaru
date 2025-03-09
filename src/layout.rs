use gtk::gdk::{Display, Monitor, Rectangle};
use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LayoutError {
    #[error("MonitorInfo error")]
    MonitorInfo,
    #[error("No display")]
    NoDisplay,
    #[error("ListModel error: {0}")]
    ListModel(String),
    #[error("Downcast error")]
    Downcast,
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

#[derive(Debug, Clone)]
struct MonitorInfo {
    connector: String,
    geometry: Rectangle,
}

impl MonitorInfo {
    fn from_monitor(monitor: &Monitor) -> Result<Self, LayoutError> {
        let connector = monitor
            .connector()
            .ok_or_else(|| LayoutError::MonitorInfo)?
            .to_string();

        Ok(Self {
            connector,
            geometry: monitor.geometry(),
        })
    }
}

fn get_monitor_info() -> Result<Vec<MonitorInfo>, LayoutError> {
    let display = Display::default().ok_or(LayoutError::NoDisplay)?;

    display
        .monitors()
        .into_iter()
        .map(|monitor| {
            let monitor = monitor.map_err(|e| LayoutError::ListModel(e.to_string()))?;
            let monitor = monitor.downcast().map_err(|_| LayoutError::Downcast)?;
            MonitorInfo::from_monitor(&monitor)
        })
        .collect()
}

fn find_primary_monitor(monitors: &[MonitorInfo]) -> Option<&MonitorInfo> {
    monitors
        .iter()
        .find(|m| m.connector == "eDP-1") // Built-in monitor
        .or_else(|| {
            monitors
                .iter()
                .rev()
                .max_by_key(|m| m.geometry.width() * m.geometry.height()) // The largest monitor
        })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultLayout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(default = "default_primary_monitor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_monitor: Option<String>,
}

fn default_primary_monitor() -> Option<String> {
    let monitors = get_monitor_info().ok()?;
    find_primary_monitor(&monitors).map(|m| m.connector.clone())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DisplayConfiguration {
    PrimarySource {
        monitor: String,
        #[serde(flatten)]
        source: SourceIdentifier,
        r#type: SourceType,
    },
    MirroredDisplay {
        monitor: String,
        mirror_of: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomLayout {
    pub configurations: Vec<DisplayConfiguration>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Layout {
    Default(DefaultLayout),
    Custom(CustomLayout),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinalizedLayout(Vec<DisplayConfiguration>);

fn validate_monitor_exists(monitors: &[MonitorInfo], name: &str) -> bool {
    monitors.iter().any(|m| m.connector == name)
}

fn validate_configuration(monitors: &[MonitorInfo], config: &DisplayConfiguration) -> bool {
    match config {
        DisplayConfiguration::PrimarySource { monitor, .. } => {
            validate_monitor_exists(monitors, monitor)
        }
        DisplayConfiguration::MirroredDisplay { monitor, mirror_of } => {
            validate_monitor_exists(monitors, monitor)
                && validate_monitor_exists(monitors, mirror_of)
        }
    }
}

pub fn finalize_layout(layout: Layout) -> Result<FinalizedLayout, LayoutError> {
    let monitors = get_monitor_info()?;

    match layout {
        Layout::Default(layout) => {
            let primary_monitor = layout
                .primary_monitor
                .as_ref()
                .filter(|name| validate_monitor_exists(&monitors, name))
                .or_else(|| find_primary_monitor(&monitors).map(|m| &m.connector));

            let primary_monitor = match primary_monitor {
                Some(m) => m,
                None => return Ok(FinalizedLayout(vec![])),
            };

            let mut configurations = Vec::new();

            if let Some(source) = layout.source {
                configurations.push(DisplayConfiguration::PrimarySource {
                    monitor: primary_monitor.clone(),
                    source: source.identifier,
                    r#type: source.r#type,
                });
            } else {
                return Ok(FinalizedLayout(vec![]));
            }

            let mirror_configs = monitors
                .iter()
                .filter(|m| m.connector != *primary_monitor)
                .map(|m| DisplayConfiguration::MirroredDisplay {
                    monitor: m.connector.clone(),
                    mirror_of: primary_monitor.clone(),
                });

            configurations.extend(mirror_configs);

            Ok(FinalizedLayout(configurations))
        }
        Layout::Custom(layout) => {
            let valid_configs = layout
                .configurations
                .into_iter()
                .filter(|cfg| validate_configuration(&monitors, cfg))
                .collect();

            Ok(FinalizedLayout(valid_configs))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::Once;

    static INIT: Once = Once::new();

    fn initialize() {
        INIT.call_once(|| {
            gtk::init().unwrap();
        });
    }

    #[test]
    fn print_monitors() {
        initialize();

        get_monitor_info().unwrap().iter().for_each(|monitor| {
            println!("Monitor: {:?}, {:?}", monitor.connector, monitor.geometry);
        });
    }

    #[test]
    fn test_default_layout_to_json() {
        initialize();

        let default_layout = DefaultLayout {
            source: Some(Source {
                identifier: SourceIdentifier::Filepath {
                    filepath: "./test.webm".to_string(),
                },
                r#type: SourceType::Video,
            }),
            primary_monitor: None,
        };
        let default_layout = serde_json::to_string_pretty(&default_layout).unwrap();
        println!("{}", default_layout);
    }

    #[test]
    fn test_finalize_default_layout() {
        initialize();

        let json = r#"{
            "source": {
                "filepath": "./test.webm",
                "type": "video"
            }
        }"#;
        let default_layout: DefaultLayout = serde_json::from_str(json).unwrap();
        let layout = Layout::Default(default_layout);
        let final_layout = finalize_layout(layout);
        println!("{:?}", final_layout);
    }

    #[test]
    fn test_finalize_default_layout_with_primary_monitor() {
        initialize();

        let json = r#"{
            "source": {
                "uri": "https://jeffshee.github.io/herta-wallpaper/",
                "type": "web"
            },
            "primary_monitor": "eDP-1"
        }"#;
        let default_layout: DefaultLayout = serde_json::from_str(json).unwrap();
        let layout = Layout::Default(default_layout);
        let final_layout = finalize_layout(layout);
        println!("{:?}", final_layout);
    }

    #[test]
    fn test_finalize_default_layout_empty() {
        initialize();

        let json = r#"{}"#;
        let default_layout: DefaultLayout = serde_json::from_str(json).unwrap();
        let layout = Layout::Default(default_layout);
        let final_layout = finalize_layout(layout).unwrap();
        println!("{:?}", final_layout);
        assert_eq!(final_layout, FinalizedLayout(vec![]));
    }

    #[test]
    fn test_custom_layout_to_json() {
        initialize();

        let configurations = vec![
            DisplayConfiguration::PrimarySource {
                monitor: "DP-4".to_string(),
                source: SourceIdentifier::Filepath {
                    filepath: "./test.webm".to_string(),
                },
                r#type: SourceType::Video,
            },
            DisplayConfiguration::MirroredDisplay {
                monitor: "DP-2".to_string(),
                mirror_of: "DP-4".to_string(),
            },
        ];
        let custom_layout = serde_json::to_string_pretty(&CustomLayout { configurations }).unwrap();
        println!("{}", custom_layout);
    }

    #[test]
    fn test_finalize_custom_layout() {
        initialize();

        let json = r#"{
            "configurations": [
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
        let final_layout = finalize_layout(custom_layout);
        println!("{:?}", final_layout);
    }

    #[test]
    fn test_finalize_custom_layout_multi() {
        initialize();

        let json = r#"{
            "configurations": [
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
        let final_layout = finalize_layout(custom_layout).unwrap();
        println!("{:?}", final_layout);
    }

    #[test]
    fn test_finalize_custom_layout_empty() {
        initialize();

        let json = r#"{
            "configurations": []
        }"#;
        let custom_layout: CustomLayout = serde_json::from_str(json).unwrap();
        let custom_layout = Layout::Custom(custom_layout);
        let final_layout = finalize_layout(custom_layout).unwrap();
        println!("{:?}", final_layout);
        assert_eq!(final_layout, FinalizedLayout(vec![]));
    }
}
