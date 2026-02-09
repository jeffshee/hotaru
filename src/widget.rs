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

mod clapper;
mod gstgtk4;
mod web;

use enum_dispatch::enum_dispatch;
use gtk::Widget;

use crate::model::WallpaperType;

pub use clapper::ClapperWidget;
pub use gstgtk4::GstGtk4Widget;
pub use web::WebWidget;

pub trait RendererWidgetBuilder {
    fn with_filepath(filepath: &str) -> Self;
    fn with_uri(uri: &str) -> Self;
}

#[enum_dispatch]
pub trait RendererWidget: AsRef<Widget> {
    fn mirror(&self, enable_graphics_offload: bool) -> gtk::Box;
    fn play(&self);
    fn pause(&self);
    fn stop(&self);
    fn set_volume(&self, volume: f64);
    fn set_mute(&self, mute: bool);
    fn set_content_fit(&self, fit: gtk::ContentFit);
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
    pub fn with_filepath(
        filepath: &str,
        wallpaper_type: &WallpaperType,
        use_clapper: bool,
        enable_graphics_offload: bool,
    ) -> Self {
        match wallpaper_type {
            WallpaperType::Video => {
                if use_clapper {
                    Self::Clapper(ClapperWidget::with_filepath(filepath))
                } else {
                    Self::GstGtk4(GstGtk4Widget::with_filepath(
                        filepath,
                        enable_graphics_offload,
                    ))
                }
            }
            WallpaperType::Web => Self::Web(WebWidget::with_filepath(filepath)),
        }
    }

    pub fn with_uri(
        uri: &str,
        wallpaper_type: &WallpaperType,
        use_clapper: bool,
        enable_graphics_offload: bool,
    ) -> Self {
        match wallpaper_type {
            WallpaperType::Video => {
                if use_clapper {
                    Self::Clapper(ClapperWidget::with_uri(uri))
                } else {
                    Self::GstGtk4(GstGtk4Widget::with_uri(uri, enable_graphics_offload))
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
