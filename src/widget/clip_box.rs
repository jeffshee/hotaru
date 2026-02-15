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

//! A container widget that clips its child to a given viewport region.
//!
//! The child is allocated at full canvas size and translated so that the
//! viewport region appears at (0, 0). The widget itself reports only the
//! viewport (window) size during measurement, preventing the oversized
//! child from inflating the parent window.

use glib::Object;
use glib::subclass::types::ObjectSubclassIsExt as _;
use gtk::{glib, graphene, gsk, prelude::*};

glib::wrapper! {
    pub struct ClipBox(ObjectSubclass<imp::ClipBox>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ClipBox {
    /// Create a new `ClipBox`.
    ///
    /// * `child` – the widget to display (e.g. a renderer)
    /// * `window_width`, `window_height` – the size this widget reports
    /// * `canvas_width`, `canvas_height` – the actual allocation for `child`
    /// * `offset_x`, `offset_y` – translation applied to `child`
    pub fn new(
        child: &gtk::Widget,
        window_width: i32,
        window_height: i32,
        canvas_width: i32,
        canvas_height: i32,
        offset_x: i32,
        offset_y: i32,
    ) -> Self {
        let obj: Self = Object::builder().build();
        let imp = obj.imp();
        imp.window_width.set(window_width);
        imp.window_height.set(window_height);
        imp.canvas_width.set(canvas_width);
        imp.canvas_height.set(canvas_height);
        imp.offset_x.set(offset_x);
        imp.offset_y.set(offset_y);
        child.set_parent(&obj);
        obj
    }
}

mod imp {
    use super::*;
    use gtk::subclass::prelude::*;
    use std::cell::Cell;

    #[derive(Default)]
    pub struct ClipBox {
        pub(super) window_width: Cell<i32>,
        pub(super) window_height: Cell<i32>,
        pub(super) canvas_width: Cell<i32>,
        pub(super) canvas_height: Cell<i32>,
        pub(super) offset_x: Cell<i32>,
        pub(super) offset_y: Cell<i32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ClipBox {
        const NAME: &'static str = "ClipBox";
        type Type = super::ClipBox;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for ClipBox {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }

    impl WidgetImpl for ClipBox {
        fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
            // Report only the viewport (window) size, not the full canvas.
            let size = match orientation {
                gtk::Orientation::Horizontal => self.window_width.get(),
                gtk::Orientation::Vertical => self.window_height.get(),
                _ => 0,
            };
            (size, size, -1, -1)
        }

        fn size_allocate(&self, _width: i32, _height: i32, _baseline: i32) {
            if let Some(child) = self.obj().first_child() {
                let transform = gsk::Transform::new().translate(&graphene::Point::new(
                    -self.offset_x.get() as f32,
                    -self.offset_y.get() as f32,
                ));
                child.allocate(
                    self.canvas_width.get(),
                    self.canvas_height.get(),
                    -1,
                    Some(transform),
                );
            }
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let obj = self.obj();
            // Clip rendering to the viewport size.
            snapshot.push_clip(&graphene::Rect::new(
                0.0,
                0.0,
                self.window_width.get() as f32,
                self.window_height.get() as f32,
            ));
            if let Some(child) = obj.first_child() {
                obj.snapshot_child(&child, snapshot);
            }
            snapshot.pop();
        }
    }
}
