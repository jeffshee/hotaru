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

//! Process-wide OpenGL symbol resolver shared by the GL-based renderers
//! (libmpv, wallpaper-engine scenes). Both libraries resolve every GL
//! function through a caller-provided loader; GTK exposes no public one, so
//! resolve via eglGetProcAddress or glXGetProcAddressARB depending on which
//! platform GDK actually realized its GL context on.

use std::ffi::{c_char, c_void, CString};
use std::ptr;
use std::sync::OnceLock;

use gtk::{glib, prelude::*};
use tracing::{error, info};

type GlGetProcAddressFn = unsafe extern "C" fn(*const c_char) -> *mut c_void;
type GlGetIntegervFn = unsafe extern "C" fn(u32, *mut i32);

const GL_FRAMEBUFFER_BINDING: u32 = 0x8CA6;

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
/// (it inspects the GDK display) before the first symbol lookup.
pub(crate) fn init_gl_resolver() {
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
        info!("GL resolver initialized via {}", lib_name);
        Some(GlResolver {
            _lib: lib,
            get_proc_address,
            gl_get_integerv,
        })
    });
}

/// Resolve a GL symbol from a C string, suitable for C callback trampolines.
pub(crate) fn get_proc_address_cstr(name: *const c_char) -> *mut c_void {
    let Some(resolver) = GL_RESOLVER.get().and_then(Option::as_ref) else {
        return ptr::null_mut();
    };
    unsafe { (resolver.get_proc_address)(name) }
}

/// Resolve a GL symbol by name, suitable for Rust-side loader callbacks.
pub(crate) fn get_proc_address_str(name: &str) -> *mut c_void {
    let Ok(name) = CString::new(name) else {
        return ptr::null_mut();
    };
    get_proc_address_cstr(name.as_ptr())
}

/// The FBO GTK bound for the GLArea; renderers must draw into it, not 0.
pub(crate) fn current_framebuffer_binding() -> i32 {
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
