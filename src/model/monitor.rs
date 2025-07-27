use std::collections::HashMap;

use gtk::{gdk::Monitor, gio::ListModel, prelude::*};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MonitorError {
    #[error("No display")]
    NoDisplay,
    #[error("Monitor error: {0}")]
    MonitorError(String),
    #[error("ListModel error: {0}")]
    MonitorListModel(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonitorInfo {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl From<&Monitor> for MonitorInfo {
    fn from(monitor: &Monitor) -> Self {
        Self {
            x: monitor.geometry().x(),
            y: monitor.geometry().y(),
            height: monitor.geometry().height(),
            width: monitor.geometry().width(),
        }
    }
}

pub type MonitorMap = HashMap<String, MonitorInfo>;
pub trait MonitorListModelExt {
    fn try_to_monitor_vec(&self) -> Result<Vec<Monitor>, MonitorError>;
    fn try_to_monitor_map(&self) -> Result<MonitorMap, MonitorError>;
}

impl MonitorListModelExt for ListModel {
    fn try_to_monitor_vec(&self) -> Result<Vec<Monitor>, MonitorError> {
        self.into_iter()
            .map(|item| {
                item.map_err(|e| MonitorError::MonitorListModel(e.to_string()))?
                    .downcast::<Monitor>()
                    .map_err(|o| {
                        MonitorError::MonitorListModel(format!(
                            "Failed to downcast object to gdk::Monitor, object: {:?}",
                            o
                        ))
                    })
            })
            .collect()
    }

    fn try_to_monitor_map(&self) -> Result<MonitorMap, MonitorError> {
        Ok(self
            .try_to_monitor_vec()?
            .iter()
            .map(|m| {
                let connector = match m.connector() {
                    Some(connector) => connector.to_string(),
                    None => "Unknown".to_string(),
                };

                (connector, m.into())
            })
            .collect())
    }
}
