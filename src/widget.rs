mod clapper;
mod gstgtk4;
mod web;

use enum_dispatch::enum_dispatch;
use gtk::Widget;

use crate::layout::WallpaperType;
pub use clapper::*;
pub use gstgtk4::*;
pub use web::*;

pub trait RendererWidgetBuilder {
    fn with_filepath(filepath: &str) -> Self;
    fn with_uri(uri: &str) -> Self;
}

#[enum_dispatch]
pub trait RendererWidget: AsRef<Widget> {
    fn mirror(&self) -> gtk::Box;
    fn play(&self);
    fn pause(&self);
    fn stop(&self);
    fn widget(&self) -> &Widget {
        self.as_ref()
    }
}

#[enum_dispatch(RendererWidget)]
#[derive(Debug)]
#[non_exhaustive]
pub enum Renderer {
    Clapper(ClapperWidget),
    Web(WebWidget),
    GstGtk4(GstGtk4Widget),
}

impl Renderer {
    pub fn with_filepath(filepath: &str, wallpaper_type: &WallpaperType, use_clapper: bool) -> Self {
        match wallpaper_type {
            WallpaperType::Video => {
                if use_clapper {
                    Self::Clapper(ClapperWidget::with_filepath(filepath))
                } else {
                    Self::GstGtk4(GstGtk4Widget::with_filepath(filepath))
                }
            }
            WallpaperType::Web => Self::Web(WebWidget::with_filepath(filepath)),
        }
    }

    pub fn with_uri(uri: &str, wallpaper_type: &WallpaperType, use_clapper: bool) -> Self {
        match wallpaper_type {
            WallpaperType::Video => {
                if use_clapper {
                    Self::Clapper(ClapperWidget::with_uri(uri))
                } else {
                    Self::GstGtk4(GstGtk4Widget::with_uri(uri))
                }
            }
            WallpaperType::Web => Self::Web(WebWidget::with_uri(uri)),
        }
    }
}

impl AsRef<Widget> for Renderer {
    fn as_ref(&self) -> &Widget {
        RendererWidget::widget(self)
    }
}
