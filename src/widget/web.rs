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

use glib::Object;
use gtk::{gio, glib, prelude::*};

use super::{RendererWidget, RendererWidgetBuilder};

glib::wrapper! {
    pub struct WebWidget(ObjectSubclass<imp::WebWidget>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RendererWidgetBuilder for WebWidget {
    fn with_filepath(filepath: &str) -> Self {
        let uri = gio::File::for_path(filepath).uri();
        Object::builder()
            .property("uri", uri.as_str())
            .property("sandbox-path", parent_dir(filepath))
            .build()
    }

    fn with_uri(uri: &str) -> Self {
        Object::builder().property("uri", uri).build()
    }
}

impl WebWidget {
    /// Build a web wallpaper for a Wallpaper Engine package rooted at
    /// `package_dir`: load `filepath` and deliver `properties` (JSON
    /// `{name:{value:…}}`) to the wallpaper's `applyUserProperties` once
    /// loaded. The package directory is granted into WebKit's sandbox so the
    /// wallpaper's bundled media plays (see `sandbox-path`).
    pub fn with_wpe(filepath: &str, properties: &str, package_dir: &str) -> Self {
        let uri = gio::File::for_path(filepath).uri();
        Object::builder()
            .property("uri", uri.as_str())
            .property("wpe-properties", properties)
            .property("sandbox-path", package_dir)
            .build()
    }
}

/// The absolute parent directory of `filepath`, for sandbox grants.
fn parent_dir(filepath: &str) -> String {
    let path = std::fs::canonicalize(filepath).unwrap_or_else(|_| filepath.into());
    path.parent()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl RendererWidget for WebWidget {
    fn mirror(&self, enable_graphics_offload: bool, content_fit: gtk::ContentFit) -> gtk::Box {
        let widget = gtk::Box::builder().build();
        let paintable = gtk::WidgetPaintable::new(Some(&self.webview()));
        let picture = gtk::Picture::builder()
            .paintable(&paintable)
            .hexpand(true)
            .vexpand(true)
            .content_fit(content_fit)
            .build();
        if enable_graphics_offload {
            let offload = gtk::GraphicsOffload::new(Some(&picture));
            offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
            widget.append(&offload);
        } else {
            widget.append(&picture);
        }
        widget
    }

    fn play(&self) {}

    fn pause(&self) {}

    fn stop(&self) {}

    fn set_volume(&self, _volume: i32) {}

    fn set_mute(&self, _mute: bool) {}

    fn set_content_fit(&self, _fit: gtk::ContentFit) {}
}

mod imp {
    use super::*;

    use std::cell::RefCell;

    use glib::Properties;
    use gtk::subclass::prelude::*;
    use tracing::debug;
    use webkit::{prelude::*, WebView};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::WebWidget)]
    pub struct WebWidget {
        #[property(get, set)]
        uri: RefCell<String>,
        /// Wallpaper Engine user properties as a JSON object
        /// (`{name:{value:…}}`), delivered to the wallpaper's
        /// `applyUserProperties` after load. Empty for non-WPE web wallpapers.
        #[property(get, set, name = "wpe-properties")]
        wpe_properties: RefCell<String>,
        /// Directory granted read-only into WebKit's sandbox. Local media
        /// (`<audio>`/`<video>`) is read by GStreamer `filesrc` inside the
        /// sandboxed WebProcess — unlike page subresources, which the
        /// unsandboxed NetworkProcess fetches — so without this grant a
        /// wallpaper's bundled media fails with SRC_NOT_SUPPORTED. Must be
        /// set at construction: sandbox grants are immutable once the web
        /// process has spawned. Empty for remote (non-file) wallpapers.
        #[property(get, set, construct, name = "sandbox-path")]
        sandbox_path: RefCell<String>,
        #[property(get)]
        webview: RefCell<WebView>,
    }

    /// Minimal Wallpaper Engine JS API, injected at document-start so web
    /// wallpapers that call these globals don't throw. Property delivery
    /// (`applyUserProperties`) happens after load, from Rust.
    const WPE_API_STUB: &str = r#"
(function () {
  if (window.__hotaruWpeStub) return;
  window.__hotaruWpeStub = true;
  var noop = function () {};
  // Feed a zeroed 128-sample spectrum (64 L + 64 R) so audio-reactive
  // wallpapers run (flat, not reactive — hotaru has no spectrum feed).
  window.wallpaperRegisterAudioListener = function (cb) {
    if (typeof cb !== 'function') return;
    var silent = new Array(128).fill(0);
    if (window.__hotaruAudioTimer) clearInterval(window.__hotaruAudioTimer);
    window.__hotaruAudioTimer = setInterval(function () {
      try { cb(silent); } catch (e) {}
    }, 33);
  };
  window.wallpaperRequestRandomFileForProperty = noop;
  window.wallpaperRegisterMediaStatusListener = noop;
  window.wallpaperRegisterMediaPropertiesListener = noop;
  window.wallpaperRegisterMediaThumbnailListener = noop;
  window.wallpaperRegisterMediaTimelineListener = noop;
  window.wallpaperRegisterMediaPlaybackListener = noop;
})();
"#;

    impl WebWidget {
        pub fn start(&self) {
            debug!("start {}", self.uri.borrow());
            self.webview.borrow().load_uri(&self.uri.borrow());
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WebWidget {
        const NAME: &'static str = "WebWidget";
        type Type = super::WebWidget;
        type ParentType = gtk::Box;
    }

    #[glib::derived_properties]
    impl ObjectImpl for WebWidget {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            // Inject the Wallpaper Engine JS API stub before any page script
            // runs, so WPE web wallpapers that call these globals don't throw.
            let content_manager = webkit::UserContentManager::new();
            content_manager.add_script(&webkit::UserScript::new(
                WPE_API_STUB,
                webkit::UserContentInjectedFrames::AllFrames,
                webkit::UserScriptInjectionTime::Start,
                &[],
                &[],
            ));
            // WPE web wallpapers autoplay bundled audio/video (e.g. album
            // wallpapers); WebKit blocks autoplay with sound by default
            // (requires a user gesture). Allow it, matching WPE's browser.
            let policies = webkit::WebsitePolicies::builder()
                .autoplay(webkit::AutoplayPolicy::Allow)
                .build();
            // A per-widget WebContext, so the wallpaper's directory can be
            // granted into this widget's sandbox (see `sandbox-path`). The
            // grant must precede the first load — a web process only inherits
            // paths granted before it spawns.
            let context = webkit::WebContext::new();
            let sandbox_path = self.sandbox_path.borrow();
            if !sandbox_path.is_empty() {
                debug!("granting WebKit sandbox read access to {}", sandbox_path);
                context.add_path_to_sandbox(std::path::Path::new(&*sandbox_path), true);
            }
            drop(sandbox_path);
            let webview = WebView::builder()
                .web_context(&context)
                .user_content_manager(&content_manager)
                .website_policies(&policies)
                .build();

            // WPE web wallpapers load local assets (Spine skeletons/atlases,
            // textures, JSON) via XHR/fetch, which WebKit blocks for file://
            // origins by default — so such wallpapers render only partially
            // (e.g. cursor/canvas effects but no character). Allow local file
            // access, matching the browser environment Wallpaper Engine runs
            // them in.
            let settings = webkit::Settings::new();
            settings.set_allow_file_access_from_file_urls(true);
            settings.set_allow_universal_access_from_file_urls(true);
            // WPE web wallpapers are commonly WebGL (Spine, canvas); force
            // hardware-accelerated compositing for correct/smooth rendering.
            settings.set_hardware_acceleration_policy(webkit::HardwareAccelerationPolicy::Always);
            // Media (incl. each new <audio> a playlist creates) may play
            // without a user gesture — a desktop wallpaper never gets one.
            settings.set_media_playback_requires_user_gesture(false);
            // Opt-in: route the wallpaper's JS console to stdout for debugging
            // (HOTARU_WEB_CONSOLE=1). Web wallpapers have no visible console.
            if std::env::var_os("HOTARU_WEB_CONSOLE").is_some() {
                settings.set_enable_write_console_messages_to_stdout(true);
            }
            webview.set_settings(&settings);

            // Once the page has loaded, hand the wallpaper its properties the
            // way Wallpaper Engine does — this is what drives property-gated
            // rendering (e.g. which model/quality to load).
            webview.connect_load_changed(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |webview, event| {
                    if event != webkit::LoadEvent::Finished {
                        return;
                    }
                    let props = imp.wpe_properties.borrow();
                    if props.is_empty() {
                        return;
                    }
                    let js = format!(
                        "(function(){{var l=window.wallpaperPropertyListener;if(!l)return;\
                         if(l.applyGeneralProperties)l.applyGeneralProperties({{fps:60}});\
                         if(l.applyUserProperties)l.applyUserProperties({props});}})();",
                        props = &*props
                    );
                    webview.evaluate_javascript(
                        &js,
                        None,
                        None,
                        gio::Cancellable::NONE,
                        |_result| {},
                    );
                }
            ));

            webview.set_hexpand(true);
            webview.set_vexpand(true);
            obj.append(&webview);

            obj.connect_uri_notify(|obj| {
                obj.webview().load_uri(&obj.uri());
            });

            self.webview.replace(webview);
        }
    }

    impl WidgetImpl for WebWidget {}

    impl BoxImpl for WebWidget {}
}
