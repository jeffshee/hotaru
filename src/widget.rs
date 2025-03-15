mod clapper;
mod gstgtk4;
mod web;

use crate::layout::SourceType;
use enum_dispatch::enum_dispatch;
use gtk::Widget;

pub use clapper::*;
pub use gstgtk4::*;
pub use web::*;

const USE_CLAPPER: bool = true;

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
    pub fn with_filepath(filepath: &str, r#type: &SourceType) -> Self {
        match r#type {
            SourceType::Video => match USE_CLAPPER {
                true => Self::Clapper(ClapperWidget::with_filepath(filepath)),
                false => Self::GstGtk4(GstGtk4Widget::with_filepath(filepath)),
            },
            SourceType::Web => Self::Web(WebWidget::with_filepath(filepath)),
        }
    }

    pub fn with_uri(uri: &str, r#type: &SourceType) -> Self {
        match r#type {
            SourceType::Video => match USE_CLAPPER {
                true => Self::Clapper(ClapperWidget::with_uri(uri)),
                false => Self::GstGtk4(GstGtk4Widget::with_uri(uri)),
            },
            SourceType::Web => Self::Web(WebWidget::with_uri(uri)),
        }
    }
}

impl AsRef<Widget> for Renderer {
    fn as_ref(&self) -> &Widget {
        RendererWidget::widget(self)
    }
}
