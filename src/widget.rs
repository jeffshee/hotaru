mod clapper;
mod gstgtk4;
mod web;

use gtk::glib::object::{Cast as _, IsA};
use gtk::Widget;

pub use clapper::*;
pub use gstgtk4::*;
pub use web::*;

const USE_CLAPPER: bool = true;

use crate::layout::SourceType;

pub trait RendererWidgetBuilder {
    fn with_filepath(filepath: &str) -> Self;
    fn with_uri(uri: &str) -> Self;
}

pub trait RendererWidget {
    fn mirror(&self) -> gtk::Box;
    fn play(&self);
    fn pause(&self);
    fn stop(&self);
}

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

    pub fn as_widget(&self) -> &impl IsA<Widget> {
        match self {
            Renderer::Clapper(clapper_widget) => clapper_widget.upcast_ref::<Widget>(),
            Renderer::Web(web_widget) => web_widget.upcast_ref(),
            Renderer::GstGtk4(gst_gtk4_widget) => gst_gtk4_widget.upcast_ref(),
        }
    }
}

impl RendererWidget for Renderer {
    fn mirror(&self) -> gtk::Box {
        match self {
            Renderer::Clapper(clapper_widget) => clapper_widget.mirror(),
            Renderer::Web(web_widget) => web_widget.mirror(),
            Renderer::GstGtk4(gst_gtk4_widget) => gst_gtk4_widget.mirror(),
        }
    }

    fn play(&self) {
        match self {
            Renderer::Clapper(clapper_widget) => clapper_widget.play(),
            Renderer::Web(web_widget) => web_widget.play(),
            Renderer::GstGtk4(gst_gtk4_widget) => gst_gtk4_widget.play(),
        }
    }

    fn pause(&self) {
        match self {
            Renderer::Clapper(clapper_widget) => clapper_widget.pause(),
            Renderer::Web(web_widget) => web_widget.pause(),
            Renderer::GstGtk4(gst_gtk4_widget) => gst_gtk4_widget.pause(),
        }
    }

    fn stop(&self) {
        match self {
            Renderer::Clapper(clapper_widget) => clapper_widget.stop(),
            Renderer::Web(web_widget) => web_widget.stop(),
            Renderer::GstGtk4(gst_gtk4_widget) => gst_gtk4_widget.stop(),
        }
    }
}
