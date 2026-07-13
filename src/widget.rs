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

mod clip_box;
#[cfg(any(feature = "mpv", feature = "scene"))]
mod gl_loader;
mod gstgtk4;
#[cfg(feature = "mpv")]
mod mpv;
#[cfg(feature = "scene")]
mod scene;
mod web;

use enum_dispatch::enum_dispatch;
use gtk::Widget;

use crate::model::{VideoRenderer, WallpaperType};

pub use clip_box::ClipBox;
pub use gstgtk4::GstGtk4Widget;
#[cfg(feature = "mpv")]
pub use mpv::MpvWidget;
#[cfg(feature = "scene")]
pub use scene::SceneWidget;
pub use web::WebWidget;

pub trait RendererWidgetBuilder {
    fn with_filepath(filepath: &str) -> Self;
    fn with_uri(uri: &str) -> Self;
}

#[enum_dispatch]
pub trait RendererWidget: AsRef<Widget> {
    fn mirror(&self, enable_graphics_offload: bool, content_fit: gtk::ContentFit) -> gtk::Box;
    fn play(&self);
    fn pause(&self);
    fn stop(&self);
    /// Set the audio volume (0-100).
    fn set_volume(&self, volume: i32);
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
    Web(WebWidget),
    GstGtk4(GstGtk4Widget),
    #[cfg(feature = "mpv")]
    Mpv(MpvWidget),
    #[cfg(feature = "scene")]
    Scene(SceneWidget),
}

impl Renderer {
    pub fn with_filepath(
        filepath: &str,
        wallpaper_type: &WallpaperType,
        video_renderer: VideoRenderer,
        enable_graphics_offload: bool,
    ) -> Self {
        match wallpaper_type {
            WallpaperType::Video => match resolve_video_renderer(video_renderer) {
                VideoRenderer::GstGtk4 => Self::GstGtk4(GstGtk4Widget::with_filepath(
                    filepath,
                    enable_graphics_offload,
                )),
                #[cfg(feature = "mpv")]
                VideoRenderer::Mpv => Self::Mpv(MpvWidget::with_filepath(filepath)),
                #[cfg(not(feature = "mpv"))]
                VideoRenderer::Mpv => unreachable!(),
            },
            WallpaperType::Web => Self::Web(WebWidget::with_filepath(filepath)),
            #[cfg(feature = "scene")]
            WallpaperType::Scene => Self::Scene(SceneWidget::with_filepath(filepath)),
            #[cfg(not(feature = "scene"))]
            WallpaperType::Scene => scene_unsupported(),
        }
    }

    pub fn with_uri(
        uri: &str,
        wallpaper_type: &WallpaperType,
        video_renderer: VideoRenderer,
        enable_graphics_offload: bool,
    ) -> Self {
        match wallpaper_type {
            WallpaperType::Video => match resolve_video_renderer(video_renderer) {
                VideoRenderer::GstGtk4 => {
                    Self::GstGtk4(GstGtk4Widget::with_uri(uri, enable_graphics_offload))
                }
                #[cfg(feature = "mpv")]
                VideoRenderer::Mpv => Self::Mpv(MpvWidget::with_uri(uri)),
                #[cfg(not(feature = "mpv"))]
                VideoRenderer::Mpv => unreachable!(),
            },
            WallpaperType::Web => Self::Web(WebWidget::with_uri(uri)),
            #[cfg(feature = "scene")]
            WallpaperType::Scene => Self::Scene(SceneWidget::with_uri(uri)),
            #[cfg(not(feature = "scene"))]
            WallpaperType::Scene => scene_unsupported(),
        }
    }
}

/// Placeholder for scene wallpapers in builds without the 'scene' feature.
#[cfg(not(feature = "scene"))]
fn scene_unsupported() -> Renderer {
    tracing::error!(
        "scene wallpaper requested but this build lacks the 'scene' feature, \
         showing a blank wallpaper"
    );
    Renderer::Web(WebWidget::with_uri("about:blank"))
}

/// Downgrade renderer choices this build cannot honor.
fn resolve_video_renderer(video_renderer: VideoRenderer) -> VideoRenderer {
    #[cfg(not(feature = "mpv"))]
    if video_renderer == VideoRenderer::Mpv {
        tracing::warn!(
            "mpv renderer requested but this build lacks the 'mpv' feature, \
             falling back to GStreamer"
        );
        return VideoRenderer::GstGtk4;
    }
    video_renderer
}

impl AsRef<Widget> for Renderer {
    fn as_ref(&self) -> &Widget {
        RendererWidget::widget(self)
    }
}
