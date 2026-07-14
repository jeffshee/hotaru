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
#[cfg(any(feature = "mpv", feature = "wpe"))]
mod gl_loader;
mod gstgtk4;
#[cfg(feature = "mpv")]
mod mpv;
#[cfg(feature = "wpe")]
mod scene;
mod web;

use enum_dispatch::enum_dispatch;
use gtk::Widget;

use crate::model::{VideoRenderer, WallpaperSource, WallpaperType};
use crate::wpe::{WpePackage, WpeType};

pub use clip_box::ClipBox;
pub use gstgtk4::GstGtk4Widget;
#[cfg(feature = "mpv")]
pub use mpv::MpvWidget;
#[cfg(feature = "wpe")]
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
    #[cfg(feature = "wpe")]
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
            WallpaperType::Wpe => Self::with_wpe(
                &WallpaperSource::Filepath {
                    filepath: filepath.to_string(),
                },
                video_renderer,
                enable_graphics_offload,
            ),
        }
    }

    /// Build a renderer for a Wallpaper Engine package: resolve the source,
    /// read its `project.json`, and delegate to the renderer its `type`
    /// selects — scene packages to `SceneWidget` (linux-wallpaperengine),
    /// video/web packages to hotaru's own video/web renderers.
    pub fn with_wpe(
        source: &WallpaperSource,
        video_renderer: VideoRenderer,
        enable_graphics_offload: bool,
    ) -> Self {
        let package = match WpePackage::resolve(source) {
            Ok(package) => package,
            Err(e) => {
                tracing::error!("Failed to load Wallpaper Engine package: {:#}", e);
                return blank();
            }
        };

        match package.kind {
            WpeType::Scene => {
                #[cfg(feature = "wpe")]
                {
                    Self::Scene(SceneWidget::with_filepath(&package.dir.to_string_lossy()))
                }
                #[cfg(not(feature = "wpe"))]
                {
                    scene_unsupported()
                }
            }
            WpeType::Video => match package.entry() {
                Ok(entry) => Self::with_filepath(
                    &entry.to_string_lossy(),
                    &WallpaperType::Video,
                    video_renderer,
                    enable_graphics_offload,
                ),
                Err(e) => {
                    tracing::error!("Invalid Wallpaper Engine package: {:#}", e);
                    blank()
                }
            },
            WpeType::Web => match package.entry() {
                // Web packages get the Wallpaper Engine JS API and their
                // default properties injected (see web.rs).
                Ok(entry) => Self::Web(WebWidget::with_wpe(
                    &entry.to_string_lossy(),
                    &package.user_properties_json(),
                    &package.dir.to_string_lossy(),
                )),
                Err(e) => {
                    tracing::error!("Invalid Wallpaper Engine package: {:#}", e);
                    blank()
                }
            },
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
            WallpaperType::Wpe => {
                tracing::error!(
                    "wpe wallpaper cannot be a URI ({}); use filepath or workshop_id",
                    uri
                );
                blank()
            }
        }
    }
}

/// A blank fallback renderer, used when a wallpaper cannot be constructed.
fn blank() -> Renderer {
    Renderer::Web(WebWidget::with_uri("about:blank"))
}

/// Placeholder for scene packages in builds without the 'wpe' feature.
/// (WPE video/web packages still render — only the scene backend is gated.)
#[cfg(not(feature = "wpe"))]
fn scene_unsupported() -> Renderer {
    tracing::error!(
        "scene wallpaper requested but this build lacks the 'wpe' feature, \
         showing a blank wallpaper"
    );
    blank()
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
