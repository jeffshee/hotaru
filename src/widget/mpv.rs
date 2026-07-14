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

use super::{mirror_by_snapshot, RendererWidget};

glib::wrapper! {
    pub struct MpvWidget(ObjectSubclass<imp::MpvWidget>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl MpvWidget {
    pub fn with_filepath(filepath: &str) -> Self {
        let uri = gio::File::for_path(filepath).uri();
        Self::with_uri(&uri)
    }

    pub fn with_uri(uri: &str) -> Self {
        Object::builder().property("uri", uri).build()
    }
}

impl RendererWidget for MpvWidget {
    fn mirror(&self, enable_graphics_offload: bool, content_fit: gtk::ContentFit) -> gtk::Box {
        // mpv renders straight into its GLArea and exposes no gdk::Paintable.
        mirror_by_snapshot(&self.gl_area(), enable_graphics_offload, content_fit)
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
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    use glib::Properties;
    use libmpv2::{
        render::{OpenGLInitParams, RenderContext, RenderParam, RenderParamApiType},
        Mpv, SetData,
    };
    use tracing::{debug, error, info, warn};

    use crate::widget::gl_loader::{
        current_framebuffer_binding, get_proc_address_str, init_gl_resolver,
    };

    fn get_proc_address(_ctx: &(), name: &str) -> *mut c_void {
        get_proc_address_str(name)
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
