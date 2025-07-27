use serde::{Deserialize, Serialize};

use crate::constant::HANABI_APPLICATION_ID;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HanabiWindowParams {
    pub position: [i32; 2],
    pub keep_at_bottom: bool,
    pub keep_minimized: bool,
    pub keep_position: bool,
}

impl HanabiWindowParams {
    pub fn hanabi_window_title(&self) -> String {
        let params = serde_json::to_string(&self).expect("Failed to serialize HanabiWindowParams");
        format!("@{HANABI_APPLICATION_ID}!{params}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[test]
    fn test_hanabi_window_params_default() {
        let default_params = HanabiWindowParams::default();
        assert_eq!(default_params.position, [0, 0]);
        assert!(!default_params.keep_at_bottom);
        assert!(!default_params.keep_minimized);
        assert!(!default_params.keep_position);
    }

    #[test]
    fn test_hanabi_window_params() {
        let params = HanabiWindowParams {
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

        let serialized =
            serde_json::to_value(&params).expect("Failed to serialize HanabiWindowParams");
        assert_eq!(serialized, expected_json_value);
    }

    #[test]
    fn test_hanabi_window_title() {
        let params = HanabiWindowParams {
            position: [100, 200],
            keep_at_bottom: true,
            keep_minimized: false,
            keep_position: true,
        };
        let title = params.hanabi_window_title();
        let expected_title = format!(
            "@{HANABI_APPLICATION_ID}!{{\"position\":[100,200],\"keepAtBottom\":true,\"keepMinimized\":false,\"keepPosition\":true}}"
        );
        assert_eq!(title, expected_title);
    }
}
