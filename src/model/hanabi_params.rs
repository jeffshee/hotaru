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

use serde::{Deserialize, Serialize};

use crate::constants::APPLICATION_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HanabiParams {
    pub position: [i32; 2],
    pub keep_at_bottom: bool,
    pub keep_minimized: bool,
    pub keep_position: bool,
}

impl HanabiParams {
    /// The window-title wire format the GNOME Hanabi extension matches:
    /// `@<application id>!<params json>`. The extension's title matcher
    /// must use the same application id as this build.
    pub fn window_title(&self) -> String {
        let params = serde_json::to_string(&self).expect("Failed to serialize HanabiParams");
        format!("@{APPLICATION_ID}!{params}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[test]
    fn test_hanabi_params_default() {
        let default_params = HanabiParams::default();
        assert_eq!(default_params.position, [0, 0]);
        assert!(!default_params.keep_at_bottom);
        assert!(!default_params.keep_minimized);
        assert!(!default_params.keep_position);
    }

    #[test]
    fn test_hanabi_params() {
        let params = HanabiParams {
            position: [100, 200],
            keep_at_bottom: true,
            keep_minimized: false,
            keep_position: true,
        };

        let expected_json_value = json!({
            "position": [100, 200],
            "keepAtBottom": true,
            "keepMinimized": false,
            "keepPosition": true
        });

        let serialized = serde_json::to_value(&params).expect("Failed to serialize HanabiParams");
        assert_eq!(serialized, expected_json_value);
    }

    #[test]
    fn test_hanabi_window_title() {
        let params = HanabiParams {
            position: [100, 200],
            keep_at_bottom: true,
            keep_minimized: false,
            keep_position: true,
        };
        let title = params.window_title();
        let expected_title = format!(
            "@{APPLICATION_ID}!{{\"position\":[100,200],\"keepAtBottom\":true,\"keepMinimized\":false,\"keepPosition\":true}}"
        );
        assert_eq!(title, expected_title);
    }
}
