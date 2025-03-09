use gtk::gdk::{Display, Monitor};
use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum LayoutError {
    #[error("Failed to get monitors")]
    FailedToGetMonitors,
    #[error("No primary monitor")]
    NoPrimaryMonitor,
    #[error("Layout error: {0}")]
    LayoutError(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SourceVariant {
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
    pub source: SourceVariant, // filepath or uri
    pub r#type: SourceType, // video, web
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DefaultLayout {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(default = "default_primary_monitor")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_monitor: Option<String>, // connector name, e.g. HDMI-0, DP-1
}

pub fn default_primary_monitor() -> Option<String> {
    let monitors = match get_monitors() {
        Ok(monitors) => monitors,
        Err(_) => return None,
    };

    if monitors.is_empty() {
        return None;
    }

    // Find built-in monitor or largest monitor
    let primary_monitor = monitors
        .iter()
        .find(|monitor| monitor.connector().unwrap().to_string() == "eDP-1")
        .or_else(|| {
            monitors.iter().max_by_key(|monitor| {
                let geometry = monitor.geometry();
                geometry.width() * geometry.height()
            })
        });

    primary_monitor.map(|monitor| monitor.connector().unwrap().to_string())
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FinalSource {
    Origin {
        monitor: String, // connector name, e.g. HDMI-0, DP-1
        #[serde(flatten)]
        source: SourceVariant, // filepath or uri
        r#type: SourceType, // video, web
    },
    Mirror {
        monitor: String,   // connector name, e.g. HDMI-0, DP-1
        mirror_of: String, // connector name, e.g. HDMI-0, DP-1
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CustomLayout {
    pub sources: Vec<FinalSource>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Layout {
    Default(DefaultLayout),
    Custom(CustomLayout),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FinalLayout(Vec<FinalSource>);

pub fn get_monitors() -> Result<Vec<Monitor>, LayoutError> {
    let display = Display::default().ok_or(LayoutError::FailedToGetMonitors)?;
    let monitors: Vec<Monitor> = display
        .monitors()
        .into_iter()
        .map(|monitor| {
            monitor
                .map_err(|_| LayoutError::FailedToGetMonitors)?
                .downcast::<Monitor>()
                .map_err(|_| LayoutError::FailedToGetMonitors)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(monitors)
}

fn monitor_exists(monitors: &Vec<Monitor>, monitor_name: &str) -> bool {
    monitors
        .iter()
        .any(|m| m.connector().unwrap().to_string() == monitor_name)
}

pub fn finalize_layout(layout: Layout) -> Result<FinalLayout, LayoutError> {
    let monitors: Vec<Monitor> = get_monitors()?;

    match layout {
        Layout::Default(layout) => {
            let mut final_layout = Vec::new();

            let primary_monitor = match layout.primary_monitor {
                Some(monitor) => monitor_exists(&monitors, &monitor).then_some(monitor),
                None => default_primary_monitor(),
            };

            let primary_monitor = match primary_monitor {
                Some(monitor) => monitor,
                None => return Ok(FinalLayout(vec![])),
            };

            match layout.source {
                Some(Source { source, r#type }) => {
                    final_layout.push(FinalSource::Origin {
                        monitor: primary_monitor.clone(),
                        source,
                        r#type,
                    });
                }
                None => return Ok(FinalLayout(vec![])),
            }

            monitors
                .iter()
                .filter(|monitor| {
                    monitor.connector().unwrap().to_string() != primary_monitor.clone()
                })
                .cloned()
                .for_each(|monitor| {
                    final_layout.push(FinalSource::Mirror {
                        monitor: monitor.connector().unwrap().to_string(),
                        mirror_of: primary_monitor.clone(),
                    })
                });

            Ok(FinalLayout(final_layout))
        }
        Layout::Custom(layout) => {
            let final_layout: Vec<FinalSource> = layout
                .sources
                .into_iter()
                .filter(|source| match source {
                    FinalSource::Origin { monitor, .. } => monitor_exists(&monitors, &monitor),
                    FinalSource::Mirror { monitor, mirror_of } => {
                        monitor_exists(&monitors, &monitor) && monitor_exists(&monitors, &mirror_of)
                    }
                })
                .collect::<Vec<_>>();
            Ok(FinalLayout(final_layout))
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

        get_monitors().unwrap().iter().for_each(|monitor| {
            println!("Monitor: {:?}", monitor.connector().unwrap());
        });
    }

    #[test]
    fn test_default_layout_to_json() {
        initialize();

        let default_layout = DefaultLayout {
            source: Some(Source {
                source: SourceVariant::Filepath {
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
    fn test_finalize_default_layout2() {
        initialize();

        let default_layout = DefaultLayout {
            source: Some(Source {
                source: SourceVariant::Filepath {
                    filepath: "./test.webm".to_string(),
                },
                r#type: SourceType::Video,
            }),
            primary_monitor: Some("DP-1".to_string()),
        };
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
        assert_eq!(final_layout, FinalLayout(vec![]));
    }

    #[test]
    fn test_custom_layout_to_json() {
        initialize();

        let sources = vec![
            FinalSource::Origin {
                monitor: "DP-4".to_string(),
                source: SourceVariant::Filepath {
                    filepath: "./test.webm".to_string(),
                },
                r#type: SourceType::Video,
            },
            FinalSource::Mirror {
                monitor: "DP-2".to_string(),
                mirror_of: "DP-4".to_string(),
            },
        ];
        let custom_layout = serde_json::to_string_pretty(&CustomLayout { sources }).unwrap();
        println!("{}", custom_layout);
    }

    #[test]
    fn test_finalize_custom_layout() {
        initialize();

        let json = r#"{
            "sources": [
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
            "sources": [
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
            "sources": []
        }"#;
        let custom_layout: CustomLayout = serde_json::from_str(json).unwrap();
        let custom_layout = Layout::Custom(custom_layout);
        let final_layout = finalize_layout(custom_layout).unwrap();
        println!("{:?}", final_layout);
        assert_eq!(final_layout, FinalLayout(vec![]));
    }
}
