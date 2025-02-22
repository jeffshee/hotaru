mod clapper;
mod gstgtk4;
mod web;

pub use clapper::*;
pub use gstgtk4::*;
pub use web::*;

pub trait RendererWidget {
    fn with_filepath(filepath: &str) -> Self;
    fn with_uri(uri: &str) -> Self;
    fn mirror(&self) -> gtk::Box;
    fn play(&self);
    fn pause(&self);
    fn stop(&self);
}
