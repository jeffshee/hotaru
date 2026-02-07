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
