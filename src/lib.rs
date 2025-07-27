pub mod application;
pub mod constant;
pub mod layout;
pub mod model;
pub mod monitor;
pub mod utils;
pub mod widget;
pub mod window;

pub mod prelude {
    pub use crate::application::HotaruApplication;
    pub use crate::constant::*;
    pub use crate::layout::LiveWallpaperConfig;
    pub use crate::model::LaunchMode;
    pub use crate::monitor::{MonitorListModelExt, MonitorTracker};
    pub use crate::utils::setup_gst;
}
