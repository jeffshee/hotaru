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
use gtk::{gio, glib, prelude::*, subclass::prelude::*};

use super::{RendererWidget, RendererWidgetBuilder};

glib::wrapper! {
    pub struct MpvWidget(ObjectSubclass<imp::MpvWidget>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RendererWidgetBuilder for MpvWidget {
    fn with_filepath(filepath: &str) -> Self {
        let uri = gio::File::for_path(filepath).uri();
        Self::with_uri(&uri)
    }

    fn with_uri(uri: &str) -> Self {
        Object::builder().property("uri", uri).build()
    }
}

impl RendererWidget for MpvWidget {
    fn mirror(&self, enable_graphics_offload: bool, content_fit: gtk::ContentFit) -> gtk::Box {
        // mpv renders straight into its GLArea and exposes no gdk::Paintable,
        // so mirror by snapshotting the widget, same as WebWidget.
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
        self.imp().set_mpv_property("pause", false);
    }

    fn pause(&self) {
        self.imp().set_mpv_property("pause", true);
    }

    fn stop(&self) {
        self.imp().run_mpv_command("stop", &[]);
    }

    fn set_volume(&self, volume: i32) {
        // mpv volume is also a 0-100 scale
        self.imp().set_mpv_property("volume", volume as f64);
    }

    fn set_mute(&self, mute: bool) {
        self.imp().set_mpv_property("mute", mute);
    }

    fn set_content_fit(&self, fit: gtk::ContentFit) {
        // mpv scales the video inside the GLArea itself, so content fit maps
        // to its scaling options rather than gtk::Picture properties.
        let imp = self.imp();
        match fit {
            gtk::ContentFit::Fill => {
                imp.set_mpv_property("keepaspect", false);
            }
            gtk::ContentFit::Cover => {
                imp.set_mpv_property("keepaspect", true);
                imp.set_mpv_property("panscan", 1.0);
            }
            _ => {
                imp.set_mpv_property("keepaspect", true);
                imp.set_mpv_property("panscan", 0.0);
            }
        }
    }
}

mod imp {
    use super::*;

    use std::cell::RefCell;
    use std::ffi::{c_char, c_void, CString};
    use std::ptr;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, OnceLock};

    use glib::Properties;
    use libmpv2::{
        render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType},
        Mpv, SetData,
    };
    use tracing::{debug, error, info, warn};

    type GlGetProcAddressFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
    type GlGetIntegervFn = unsafe extern "C" fn(u32, *mut i32);

    const GL_FRAMEBUFFER_BINDING: u32 = 0x8CA6;

    /// Resolves OpenGL functions for mpv. libmpv does not link GL itself and
    /// asks us for every symbol; GTK offers no public loader, so resolve via
    /// eglGetProcAddress or glXGetProcAddressARB depending on which platform
    /// GDK actually realized its GL context on.
    struct GlResolver {
        // Keeps the dlopen handle alive for the fn pointers below.
        _lib: libloading::Library,
        get_proc_address: GlGetProcAddressFn,
        gl_get_integerv: Option<GlGetIntegervFn>,
    }

    // SAFETY: the resolved fn pointers are plain C functions; the Library
    // handle is only held to keep them valid.
    unsafe impl Send for GlResolver {}
    unsafe impl Sync for GlResolver {}

    static GL_RESOLVER: OnceLock<Option<GlResolver>> = OnceLock::new();

    fn display_uses_egl() -> bool {
        let display = match gtk::gdk::Display::default() {
            Some(display) => display,
            None => return true,
        };
        // Wayland is always EGL. On X11, GDK may realize either an EGL or a
        // GLX context; gdk_x11_display_get_egl_display tells us which.
        if let Some(x11_type) = glib::Type::from_name("GdkX11Display") {
            if display.type_().is_a(x11_type) {
                if let Some(x11_display) = display.downcast_ref::<gdk_x11::X11Display>() {
                    return x11_display.egl_display().is_some();
                }
            }
        }
        true
    }

    /// Initialize the GL symbol resolver. Must be called on the main thread
    /// (it inspects the GDK display) before creating the mpv render context.
    fn init_gl_resolver() {
        GL_RESOLVER.get_or_init(|| {
            let (lib_name, sym_name): (&str, &[u8]) = if display_uses_egl() {
                ("libEGL.so.1", b"eglGetProcAddress\0")
            } else {
                ("libGL.so.1", b"glXGetProcAddressARB\0")
            };
            let lib = match unsafe { libloading::Library::new(lib_name) } {
                Ok(lib) => lib,
                Err(e) => {
                    error!("Failed to load {}: {}", lib_name, e);
                    return None;
                }
            };
            let get_proc_address = match unsafe { lib.get::<GlGetProcAddressFn>(sym_name) } {
                Ok(sym) => *sym,
                Err(e) => {
                    error!("Failed to resolve GL loader function: {}", e);
                    return None;
                }
            };
            let gl_get_integerv = unsafe {
                let ptr = get_proc_address(c"glGetIntegerv".as_ptr());
                if ptr.is_null() {
                    None
                } else {
                    Some(std::mem::transmute::<*mut c_void, GlGetIntegervFn>(ptr))
                }
            };
            info!("mpv GL resolver initialized via {}", lib_name);
            Some(GlResolver {
                _lib: lib,
                get_proc_address,
                gl_get_integerv,
            })
        });
    }

    fn get_proc_address(_ctx: &(), name: &str) -> *mut c_void {
        let Some(resolver) = GL_RESOLVER.get().and_then(Option::as_ref) else {
            return ptr::null_mut();
        };
        let Ok(name) = CString::new(name) else {
            return ptr::null_mut();
        };
        unsafe { (resolver.get_proc_address)(name.as_ptr()) }
    }

    /// The FBO GTK bound for the GLArea; mpv must render into it, not 0.
    fn current_framebuffer_binding() -> i32 {
        let mut fbo: i32 = 0;
        if let Some(gl_get_integerv) = GL_RESOLVER
            .get()
            .and_then(Option::as_ref)
            .and_then(|r| r.gl_get_integerv)
        {
            unsafe { gl_get_integerv(GL_FRAMEBUFFER_BINDING, &mut fbo) };
        }
        fbo
    }

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::MpvWidget)]
    pub struct MpvWidget {
        #[property(get, set)]
        uri: RefCell<String>,
        #[property(get, name = "gl-area")]
        gl_area: RefCell<gtk::GLArea>,
        // Declared before `mpv` so it drops first: the render context
        // borrows the mpv handle and must be freed before it.
        render_context: RefCell<Option<RenderContext<'static>>>,
        mpv: RefCell<Option<Mpv>>,
        tick_id: RefCell<Option<gtk::TickCallbackId>>,
        /// Set by mpv's render thread when a new frame is ready, polled on
        /// the frame clock to schedule a redraw on the main thread.
        needs_redraw: Arc<AtomicBool>,
    }

    impl MpvWidget {
        pub(super) fn set_mpv_property<T: SetData>(&self, name: &str, value: T) {
            if let Some(mpv) = self.mpv.borrow().as_ref() {
                if let Err(e) = mpv.set_property(name, value) {
                    warn!("Failed to set mpv property {}: {}", name, e);
                }
            }
        }

        pub(super) fn run_mpv_command(&self, name: &str, args: &[&str]) {
            if let Some(mpv) = self.mpv.borrow().as_ref() {
                if let Err(e) = mpv.command(name, args) {
                    warn!("mpv command {} failed: {}", name, e);
                }
            }
        }

        /// Load the current uri. A no-op until the render context exists;
        /// loading earlier would make mpv fail to initialize its VO.
        fn load_current_uri(&self) {
            if self.render_context.borrow().is_none() {
                return;
            }
            let uri = self.uri.borrow();
            if uri.is_empty() {
                return;
            }
            debug!("mpv loadfile: {}", uri);
            self.run_mpv_command("loadfile", &[&uri, "replace"]);
        }

        fn setup_render_context(&self, gl_area: &gtk::GLArea) {
            gl_area.make_current();
            if let Some(e) = gl_area.error() {
                error!("GLArea failed to create a GL context: {}", e);
                return;
            }
            init_gl_resolver();

            let mut render_context = {
                let mpv_guard = self.mpv.borrow();
                let Some(mpv) = mpv_guard.as_ref() else {
                    return;
                };
                let render_context = mpv.create_render_context(vec![
                    RenderParam::ApiType(RenderParamApiType::OpenGl),
                    RenderParam::InitParams(OpenGLInitParams {
                        get_proc_address,
                        ctx: (),
                    }),
                ]);
                let render_context = match render_context {
                    Ok(render_context) => render_context,
                    Err(e) => {
                        error!("Failed to create mpv render context: {}", e);
                        return;
                    }
                };
                // SAFETY: the lifetime only marks the borrow of the mpv
                // handle. Both live in this struct, the underlying mpv_handle
                // is heap-allocated (moving `Mpv` is fine), and field order
                // guarantees the render context is dropped first.
                unsafe {
                    std::mem::transmute::<RenderContext<'_>, RenderContext<'static>>(render_context)
                }
            };

            let needs_redraw = self.needs_redraw.clone();
            render_context.set_update_callback(move || {
                needs_redraw.store(true, Ordering::Release);
            });

            let needs_redraw = self.needs_redraw.clone();
            let tick_id = gl_area.add_tick_callback(move |gl_area, _clock| {
                if needs_redraw.swap(false, Ordering::AcqRel) {
                    gl_area.queue_render();
                }
                glib::ControlFlow::Continue
            });
            if let Some(old_tick) = self.tick_id.replace(Some(tick_id)) {
                old_tick.remove();
            }

            self.render_context.replace(Some(render_context));

            self.load_current_uri();
        }

        fn render(&self, gl_area: &gtk::GLArea) {
            if let Some(render_context) = self.render_context.borrow().as_ref() {
                let scale = gl_area.scale_factor();
                let width = gl_area.width() * scale;
                let height = gl_area.height() * scale;
                let fbo = current_framebuffer_binding();
                // flip = true: mpv renders y-up while GTK samples the
                // GLArea framebuffer y-down.
                if let Err(e) = render_context.render::<()>(fbo, width, height, true) {
                    warn!("mpv render failed: {}", e);
                }
            }
        }

        fn teardown_render_context(&self, gl_area: &gtk::GLArea) {
            if let Some(tick_id) = self.tick_id.take() {
                tick_id.remove();
            }
            // mpv_render_context_free needs the GL context current.
            gl_area.make_current();
            self.render_context.replace(None);
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for MpvWidget {
        const NAME: &'static str = "MpvWidget";
        type Type = super::MpvWidget;
        type ParentType = gtk::Box;
    }

    #[glib::derived_properties]
    impl ObjectImpl for MpvWidget {
        fn constructed(&self) {
            self.parent_constructed();

            info!("Using libmpv for video rendering");
            let obj = self.obj();

            let gl_area = gtk::GLArea::builder().hexpand(true).vexpand(true).build();
            obj.append(&gl_area);

            // libmpv refuses to create a handle unless LC_NUMERIC is "C",
            // but gtk::init() applies the user's locale. Force it back;
            // GTK/GLib parse numbers locale-independently anyway.
            unsafe { libc::setlocale(libc::LC_NUMERIC, c"C".as_ptr()) };

            let mpv = Mpv::with_initializer(|init| {
                // Render through the mpv render API into our GLArea.
                init.set_property("vo", "libmpv")?;
                // Wallpapers loop forever, matching the EOS handling of the
                // GStreamer-based renderers.
                init.set_property("loop-file", "inf")?;
                init.set_property("hwdec", "auto-safe")?;
                // We do not drain the mpv event queue, so surface warnings
                // and errors on stderr instead.
                init.set_property("terminal", true)?;
                init.set_property("msg-level", "all=warn")?;
                Ok(())
            });
            match mpv {
                Ok(mpv) => {
                    self.mpv.replace(Some(mpv));
                }
                Err(e) => {
                    error!("Failed to initialize mpv: {}", e);
                }
            }

            gl_area.connect_realize(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |gl_area| imp.setup_render_context(gl_area)
            ));
            gl_area.connect_unrealize(glib::clone!(
                #[weak(rename_to = imp)]
                self,
                move |gl_area| imp.teardown_render_context(gl_area)
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

            obj.connect_uri_notify(|obj| obj.imp().load_current_uri());

            self.gl_area.replace(gl_area);
        }
    }

    impl WidgetImpl for MpvWidget {}

    impl BoxImpl for MpvWidget {}
}
