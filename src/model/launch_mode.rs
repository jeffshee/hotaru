use gtk::glib::Value;
use strum_macros::{Display, EnumString};

use crate::constant::{
    LAUNCH_MODE_GNOME_EXT_HANABI, LAUNCH_MODE_WAYLAND_LAYER_SHELL, LAUNCH_MODE_WINDOWED,
    LAUNCH_MODE_X11_DESKTOP,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, EnumString, Display)]
#[strum(serialize_all = "kebab_case")]
pub enum LaunchMode {
    X11Desktop,
    WaylandLayerShell,
    GnomeExtHanabi,
    #[default]
    Windowed,
}

impl From<LaunchMode> for Value {
    fn from(value: LaunchMode) -> Self {
        match value {
            LaunchMode::X11Desktop => Value::from(LAUNCH_MODE_X11_DESKTOP),
            LaunchMode::WaylandLayerShell => Value::from(LAUNCH_MODE_WAYLAND_LAYER_SHELL),
            LaunchMode::GnomeExtHanabi => Value::from(LAUNCH_MODE_GNOME_EXT_HANABI),
            LaunchMode::Windowed => Value::from(LAUNCH_MODE_WINDOWED),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr as _;

    #[test]
    fn test_launch_mode_default() {
        let default_mode = LaunchMode::default();
        assert_eq!(default_mode, LaunchMode::Windowed);
    }

    #[test]
    fn test_window_type_from_str() {
        assert_eq!(
            LaunchMode::from_str(LAUNCH_MODE_X11_DESKTOP).unwrap(),
            LaunchMode::X11Desktop
        );
        assert_eq!(
            LaunchMode::from_str(LAUNCH_MODE_WAYLAND_LAYER_SHELL).unwrap(),
            LaunchMode::WaylandLayerShell
        );
        assert_eq!(
            LaunchMode::from_str(LAUNCH_MODE_GNOME_EXT_HANABI).unwrap(),
            LaunchMode::GnomeExtHanabi
        );
        assert_eq!(
            LaunchMode::from_str(LAUNCH_MODE_WINDOWED).unwrap(),
            LaunchMode::Windowed
        );
    }

    #[test]
    fn test_launch_mode_to_string() {
        assert_eq!(LaunchMode::X11Desktop.to_string(), LAUNCH_MODE_X11_DESKTOP);
        assert_eq!(
            LaunchMode::WaylandLayerShell.to_string(),
            LAUNCH_MODE_WAYLAND_LAYER_SHELL
        );
        assert_eq!(
            LaunchMode::GnomeExtHanabi.to_string(),
            LAUNCH_MODE_GNOME_EXT_HANABI
        );
        assert_eq!(LaunchMode::Windowed.to_string(), LAUNCH_MODE_WINDOWED);
    }

    #[test]
    fn test_launch_mode_to_glib_value() {
        let x11_value: Value = LaunchMode::X11Desktop.into();
        let wayland_value: Value = LaunchMode::WaylandLayerShell.into();
        let hanabi_value: Value = LaunchMode::GnomeExtHanabi.into();
        let windowed_value: Value = LaunchMode::Windowed.into();

        assert_eq!(x11_value.get(), Ok(String::from(LAUNCH_MODE_X11_DESKTOP)));
        assert_eq!(
            wayland_value.get(),
            Ok(String::from(LAUNCH_MODE_WAYLAND_LAYER_SHELL))
        );
        assert_eq!(
            hanabi_value.get(),
            Ok(String::from(LAUNCH_MODE_GNOME_EXT_HANABI))
        );
        assert_eq!(windowed_value.get(), Ok(String::from(LAUNCH_MODE_WINDOWED)));
    }
}
