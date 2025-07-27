pub mod application;
pub mod constant;
pub mod model;
pub mod monitor_tracker;
pub mod utils;
pub mod widget;
pub mod window;

pub mod prelude {
    pub use crate::application::HotaruApplication;
    pub use crate::constant::*;
    pub use crate::model::{
        LaunchMode, MonitorInfo, MonitorListModelExt, MonitorMap, WallpaperConfig,
    };
    pub use crate::monitor_tracker::MonitorTracker;
    pub use crate::utils::setup_gst;
}
