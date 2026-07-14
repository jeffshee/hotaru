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

//! Wallpaper Engine scene renderer, backed by linux-wallpaperengine's
//! embedding API (wpe_embed.h). The engine library is dlopen'd at runtime,
//! so builds and installs work without it; a scene wallpaper then fails
//! with a logged error instead of a startup failure.

use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use super::{RendererWidget, RendererWidgetBuilder};

glib::wrapper! {
    pub struct SceneWidget(ObjectSubclass<imp::SceneWidget>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RendererWidgetBuilder for SceneWidget {
    fn with_filepath(filepath: &str) -> Self {
        Object::builder().property("filepath", filepath).build()
    }

    fn with_uri(uri: &str) -> Self {
        // Scenes are always local directories; accept file:// URIs for
        // config symmetry with the other renderers.
        match gio::File::for_uri(uri).path() {
            Some(path) => Self::with_filepath(&path.to_string_lossy()),
            None => {
                tracing::error!("scene wallpaper requires a local path, got: {}", uri);
                Self::with_filepath("")
            }
        }
    }
}

impl RendererWidget for SceneWidget {
    fn mirror(&self, enable_graphics_offload: bool, content_fit: gtk::ContentFit) -> gtk::Box {
        // The scene renders straight into its GLArea and exposes no
        // gdk::Paintable, so mirror by snapshotting the widget, same as
        // MpvWidget and WebWidget.
        let widget = gtk::Box::builder().build();
        let paintable = gtk::WidgetPaintable::new(Some(&self.gl_area()));
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

    fn play(&self) {
        self.imp().set_paused(false);
    }

    fn pause(&self) {
        self.imp().set_paused(true);
    }

    fn stop(&self) {
        self.imp().set_paused(true);
    }

    fn set_volume(&self, volume: i32) {
        self.imp().set_volume(volume);
    }

    fn set_mute(&self, mute: bool) {
        self.imp().set_mute(mute);
    }

    fn set_content_fit(&self, fit: gtk::ContentFit) {
        self.imp().set_content_fit(fit);
    }
}

mod imp {
    use super::*;

    use std::cell::{Cell, RefCell};
    use std::ffi::{c_char, c_int, c_uint, c_void, CString};
    use std::ptr;
    use std::sync::OnceLock;

    use glib::Properties;
    use tracing::{error, info};

    use crate::widget::gl_loader::{
        current_framebuffer_binding, get_proc_address_cstr, init_gl_resolver,
    };

    /// Environment overrides: library to dlopen and Wallpaper Engine assets
    /// directory (unset = engine auto-detects a Steam install).
    const LIBRARY_ENV: &str = "HOTARU_WPE_LIBRARY";
    const ASSETS_ENV: &str = "HOTARU_WPE_ASSETS";
    const DEFAULT_LIBRARY: &str = "liblinux-wallpaperengine-lib.so";

    /// Scene render-rate cap in FPS. Kept below very high refresh rates to
    /// bound GPU use; override with `HOTARU_WPE_FPS`.
    const FPS_ENV: &str = "HOTARU_WPE_FPS";
    const DEFAULT_FPS: i64 = 60;

    fn fps_limit() -> i64 {
        static FPS: OnceLock<i64> = OnceLock::new();
        *FPS.get_or_init(|| {
            std::env::var(FPS_ENV)
                .ok()
                .and_then(|v| v.parse::<i64>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(DEFAULT_FPS)
        })
    }

    /// Embed ABI this build was compiled against (WPE_EMBED_ABI_VERSION in
    /// wpe_embed.h). The structs and signatures below are hand-mirrored from
    /// that header, so a library reporting a different version cannot be
    /// trusted — bump this in lockstep with the header.
    const WPE_ABI_VERSION: c_int = 1;

    // Mirrors wpe_init_params in wpe_embed.h.
    #[repr(C)]
    struct WpeInitParams {
        assets_dir: *const c_char,
        background: *const c_char,
        width: c_int,
        height: c_int,
        vflip: c_int,
        disable_mouse: c_int,
        disable_parallax: c_int,
        disable_audio: c_int,
        disable_audio_processing: c_int,
        volume: c_int,
        scaling: *const c_char,
        properties: *const *const c_char,
    }

    #[repr(C)]
    struct WpeContext {
        _opaque: [u8; 0],
    }

    type WpeGetProcAddressFn =
        unsafe extern "C" fn(userdata: *mut c_void, name: *const c_char) -> *mut c_void;

    struct WpeLib {
        // Keeps the dlopen handle alive for the fn pointers below.
        _lib: libloading::Library,
        create: unsafe extern "C" fn(
            *const WpeInitParams,
            WpeGetProcAddressFn,
            *mut c_void,
            *mut *mut c_char,
        ) -> *mut WpeContext,
        render: unsafe extern "C" fn(*mut WpeContext, c_uint, c_int, c_int, f64),
        set_paused: unsafe extern "C" fn(*mut WpeContext, c_int),
        set_volume: unsafe extern "C" fn(*mut WpeContext, c_int),
        set_audio_enabled: unsafe extern "C" fn(*mut WpeContext, c_int),
        set_mouse: unsafe extern "C" fn(*mut WpeContext, f64, f64, c_int, c_int),
        destroy: unsafe extern "C" fn(*mut WpeContext),
    }

    // SAFETY: plain C functions; the Library handle keeps them valid. The
    // engine itself is only ever called from the main thread.
    unsafe impl Send for WpeLib {}
    unsafe impl Sync for WpeLib {}

    static WPE_LIB: OnceLock<Option<WpeLib>> = OnceLock::new();

    fn wpe_lib() -> Option<&'static WpeLib> {
        WPE_LIB
            .get_or_init(|| {
                let lib_name =
                    std::env::var(LIBRARY_ENV).unwrap_or_else(|_| DEFAULT_LIBRARY.to_string());
                let lib = match unsafe { libloading::Library::new(&lib_name) } {
                    Ok(lib) => lib,
                    Err(e) => {
                        error!(
                            "Failed to load wallpaper engine library {} ({}); \
                             set {} to its full path",
                            lib_name, e, LIBRARY_ENV
                        );
                        return None;
                    }
                };
                macro_rules! sym {
                    ($name:literal) => {
                        match unsafe { lib.get($name) } {
                            Ok(sym) => *sym,
                            Err(e) => {
                                error!("{} lacks {:?}: {}", lib_name, $name, e);
                                return None;
                            }
                        }
                    };
                }

                // Guard the hand-mirrored ABI: a library built from a
                // different wpe_embed.h could have an incompatible
                // WpeInitParams layout, which would corrupt memory or crash.
                let abi_version: unsafe extern "C" fn() -> c_int = sym!(b"wpe_abi_version\0");
                let reported = unsafe { abi_version() };
                if reported != WPE_ABI_VERSION {
                    error!(
                        "{} reports embed ABI version {}, but this build expects {}; \
                         rebuild the library and hotaru from matching sources",
                        lib_name, reported, WPE_ABI_VERSION
                    );
                    return None;
                }

                let resolved = WpeLib {
                    create: sym!(b"wpe_context_create\0"),
                    render: sym!(b"wpe_context_render\0"),
                    set_paused: sym!(b"wpe_context_set_paused\0"),
                    set_volume: sym!(b"wpe_context_set_volume\0"),
                    set_audio_enabled: sym!(b"wpe_context_set_audio_enabled\0"),
                    set_mouse: sym!(b"wpe_context_set_mouse\0"),
                    destroy: sym!(b"wpe_context_destroy\0"),
                    _lib: lib,
                };
                info!("wallpaper engine library loaded: {}", lib_name);
                Some(resolved)
            })
            .as_ref()
    }

    unsafe extern "C" fn get_proc_address(
        _userdata: *mut c_void,
        name: *const c_char,
    ) -> *mut c_void {
        get_proc_address_cstr(name)
    }

    /// wpe volume is 0-128, hotaru's is 0-100.
    fn to_wpe_volume(volume: i32) -> c_int {
        (volume.clamp(0, 100) * 128 / 100) as c_int
    }

    fn to_wpe_scaling(fit: gtk::ContentFit) -> &'static str {
        match fit {
            gtk::ContentFit::Fill => "stretch",
            gtk::ContentFit::Contain => "fit",
            // Cover (hotaru's default) and ScaleDown
            _ => "fill",
        }
    }

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::SceneWidget)]
    pub struct SceneWidget {
        #[property(get, set)]
        filepath: RefCell<String>,
        #[property(get, name = "gl-area")]
        gl_area: RefCell<gtk::GLArea>,
        ctx: Cell<*mut WpeContext>,
        tick_id: RefCell<Option<gtk::TickCallbackId>>,
        paused: Cell<bool>,
        /// Frame-clock time (µs) of the last scheduled render, for FPS capping.
        last_render_us: Cell<i64>,
        // Cached so values set before realize (or between rebuilds) apply
        // when the engine context exists.
        volume: Cell<i32>,
        mute: Cell<bool>,
        content_fit: Cell<Option<gtk::ContentFit>>,
    }

    impl SceneWidget {
        pub(super) fn set_paused(&self, paused: bool) {
            self.paused.set(paused);
            let ctx = self.ctx.get();
            if let (Some(lib), false) = (wpe_lib(), ctx.is_null()) {
                unsafe { (lib.set_paused)(ctx, paused as c_int) };
                if !paused {
                    self.gl_area.borrow().queue_render();
                }
            }
        }

        pub(super) fn set_volume(&self, volume: i32) {
            self.volume.set(volume);
            let ctx = self.ctx.get();
            if let (Some(lib), false) = (wpe_lib(), ctx.is_null()) {
                unsafe { (lib.set_volume)(ctx, to_wpe_volume(volume)) };
            }
        }

        pub(super) fn set_mute(&self, mute: bool) {
            self.mute.set(mute);
            let ctx = self.ctx.get();
            if let (Some(lib), false) = (wpe_lib(), ctx.is_null()) {
                unsafe { (lib.set_audio_enabled)(ctx, !mute as c_int) };
            }
        }

        pub(super) fn set_content_fit(&self, fit: gtk::ContentFit) {
            if self.content_fit.replace(Some(fit)) == Some(fit) {
                return;
            }
            // The engine takes the scaling mode at scene load; rebuild the
            // context to apply a change on an already-running scene.
            if !self.ctx.get().is_null() {
                let gl_area = self.gl_area.borrow().clone();
                self.teardown_context(&gl_area);
                self.setup_context(&gl_area);
            }
        }

        fn setup_context(&self, gl_area: &gtk::GLArea) {
            let filepath = self.filepath.borrow().clone();
            if filepath.is_empty() {
                return;
            }
            let Some(lib) = wpe_lib() else {
                return;
            };

            gl_area.make_current();
            if let Some(e) = gl_area.error() {
                error!("GLArea failed to create a GL context: {}", e);
                return;
            }
            init_gl_resolver();

            let scale = gl_area.scale_factor();
            let width = (gl_area.width() * scale).max(1);
            let height = (gl_area.height() * scale).max(1);

            let assets = std::env::var(ASSETS_ENV).ok().and_then(|value| {
                if value.is_empty() {
                    None
                } else {
                    CString::new(value).ok()
                }
            });
            let Ok(background) = CString::new(filepath.as_str()) else {
                error!("invalid scene path: {}", filepath);
                return;
            };
            let scaling = to_wpe_scaling(self.content_fit.get().unwrap_or(gtk::ContentFit::Cover));
            let scaling = CString::new(scaling).unwrap();

            let params = WpeInitParams {
                assets_dir: assets.as_ref().map_or(ptr::null(), |s| s.as_ptr()),
                background: background.as_ptr(),
                width,
                height,
                // GTK samples the GLArea framebuffer bottom-up (GL texture
                // convention) while the engine's FBO output is top-down, so
                // flip the final blit (verified visually; the offscreen
                // embed-test reads rows out directly and wants vflip=0).
                vflip: 1,
                disable_mouse: 0,
                disable_parallax: 0,
                disable_audio: 0,
                disable_audio_processing: 1,
                volume: to_wpe_volume(self.volume.get()),
                scaling: scaling.as_ptr(),
                properties: ptr::null(),
            };

            let mut error_msg: *mut c_char = ptr::null_mut();
            let ctx =
                unsafe { (lib.create)(&params, get_proc_address, ptr::null_mut(), &mut error_msg) };
            if ctx.is_null() {
                let msg = if error_msg.is_null() {
                    "(no message)".to_string()
                } else {
                    let msg = unsafe { std::ffi::CStr::from_ptr(error_msg) }
                        .to_string_lossy()
                        .into_owned();
                    unsafe { libc::free(error_msg as *mut c_void) };
                    msg
                };
                error!("Failed to load scene {}: {}", filepath, msg);
                return;
            }
            info!("scene loaded: {}", filepath);
            self.ctx.set(ctx);

            unsafe {
                (lib.set_audio_enabled)(ctx, !self.mute.get() as c_int);
                (lib.set_paused)(ctx, self.paused.get() as c_int);
            }

            // Scenes animate continuously: redraw on frame clock ticks while
            // playing, capped at the FPS limit so wallpapers don't render at
            // full refresh on high-Hz displays. A paused scene stays a still
            // frame (damage events still repaint it via the render handler).
            let frame_interval_us = 1_000_000 / fps_limit();
            let tick_id = gl_area.add_tick_callback(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move |gl_area, clock| {
                    let now = clock.frame_time();
                    if !imp.paused.get() && now - imp.last_render_us.get() >= frame_interval_us {
                        imp.last_render_us.set(now);
                        gl_area.queue_render();
                    }
                    glib::ControlFlow::Continue
                }
            ));
            if let Some(old_tick) = self.tick_id.replace(Some(tick_id)) {
                old_tick.remove();
            }
        }

        fn render(&self, gl_area: &gtk::GLArea) {
            let ctx = self.ctx.get();
            let Some(lib) = wpe_lib() else {
                return;
            };
            if ctx.is_null() {
                return;
            }
            let scale = gl_area.scale_factor();
            let width = (gl_area.width() * scale).max(1);
            let height = (gl_area.height() * scale).max(1);
            let fbo = current_framebuffer_binding().max(0) as c_uint;
            let time = gl_area
                .frame_clock()
                .map(|clock| clock.frame_time() as f64 / 1_000_000.0)
                .unwrap_or_default();
            unsafe { (lib.render)(ctx, fbo, width, height, time) };
        }

        fn feed_mouse(&self, x: f64, y: f64) {
            let ctx = self.ctx.get();
            if let (Some(lib), false) = (wpe_lib(), ctx.is_null()) {
                let scale = self.gl_area.borrow().scale_factor() as f64;
                unsafe { (lib.set_mouse)(ctx, x * scale, y * scale, 0, 0) };
            }
        }

        fn teardown_context(&self, gl_area: &gtk::GLArea) {
            if let Some(tick_id) = self.tick_id.take() {
                tick_id.remove();
            }
            let ctx = self.ctx.replace(ptr::null_mut());
            if let (Some(lib), false) = (wpe_lib(), ctx.is_null()) {
                // freeing the engine's GL resources needs the context current
                gl_area.make_current();
                unsafe { (lib.destroy)(ctx) };
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SceneWidget {
        const NAME: &'static str = "SceneWidget";
        type Type = super::SceneWidget;
        type ParentType = gtk::Box;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SceneWidget {
        fn constructed(&self) {
            self.parent_constructed();

            info!("Using linux-wallpaperengine for scene rendering");
            let obj = self.obj();

            let gl_area = gtk::GLArea::builder().hexpand(true).vexpand(true).build();
            // The engine requires desktop GL 3.3 core; never let GDK pick GLES.
            gl_area.set_allowed_apis(gtk::gdk::GLAPI::GL);
            obj.append(&gl_area);

            gl_area.connect_realize(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |gl_area| imp.setup_context(gl_area)
            ));
            gl_area.connect_unrealize(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |gl_area| imp.teardown_context(gl_area)
            ));
            gl_area.connect_render(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |gl_area, _context| {
                    imp.render(gl_area);
                    glib::Propagation::Stop
                }
            ));

            // Pointer position drives scene parallax and interactive effects.
            let motion = gtk::EventControllerMotion::new();
            motion.connect_motion(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |_, x, y| imp.feed_mouse(x, y)
            ));
            gl_area.add_controller(motion);

            self.gl_area.replace(gl_area);
        }
    }

    impl WidgetImpl for SceneWidget {}

    impl BoxImpl for SceneWidget {}
}
