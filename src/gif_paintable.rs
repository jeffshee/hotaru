/* gif_paintable.rs
 *
 * Copyright 2024 Jeff Shee
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gst::glib::subclass::types::ObjectSubclassIsExt;
use gtk::gdk::Paintable;
use gtk::glib::{self, clone, Object};
use gtk::prelude::*;

glib::wrapper! {
   pub struct GifPaintable(ObjectSubclass<imp::GifPaintable>)
   @implements Paintable;
}

impl GifPaintable {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn with_filepath(filepath: &str) -> Self {
        Object::builder().property("filepath", filepath).build()
    }

    pub fn animate(&self) {
        self.imp().animate();
    }
}

mod imp {
    use super::*;
    use glib::Properties;
    use gtk::{
        gdk_pixbuf::{PixbufAnimation, PixbufAnimationIter},
        subclass::prelude::*,
    };
    use std::time::SystemTime;
    use std::{cell::RefCell, time::Duration};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::GifPaintable)]
    pub struct GifPaintable {
        #[property(get, set)]
        filepath: RefCell<String>,
        animation: RefCell<Option<PixbufAnimation>>,
        iterator: RefCell<Option<PixbufAnimationIter>>,
        delay: RefCell<Option<Duration>>,
        timeout_id: RefCell<Option<glib::SourceId>>,
    }

    impl GifPaintable {
        pub fn animate(&self) {
            let filepath = self.filepath.borrow().to_string();

            match PixbufAnimation::from_file(filepath) {
                Ok(animation) => {
                    let iterator = animation.iter(None);
                    let delay = iterator.delay_time();

                    self.animation.replace(Some(animation));
                    self.iterator.replace(Some(iterator));
                    self.delay.replace(delay);
                }
                Err(error) => {
                    eprintln!("{}", error)
                }
            }
            self.setup_timeout();
        }

        fn setup_timeout(&self) {
            let timeout_id = self.timeout_id.take();
            if let Some(timeout_id) = timeout_id {
                timeout_id.remove();
            }

            let delay = self.delay.borrow();

            if let Some(delay) = *delay {
                let timeout_id = glib::timeout_add_local(
                    delay,
                    clone!(@weak self as inner_self => @default-return glib::ControlFlow::Break, move || {
                        inner_self.obj().invalidate_contents();
                        inner_self.setup_timeout();
                        glib::ControlFlow::Break
                    }),
                );
                self.timeout_id.replace(Some(timeout_id));
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GifPaintable {
        const NAME: &'static str = "GifPaintable";
        type Type = super::GifPaintable;
        type ParentType = Object;
        type Interfaces = (Paintable,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for GifPaintable {}

    impl PaintableImpl for GifPaintable {
        fn snapshot(&self, snapshot: &gtk::gdk::Snapshot, width: f64, height: f64) {
            let iterator = self.iterator.borrow();
            if let Some(ref iterator) = *iterator {
                iterator.advance(SystemTime::now());
                let pixbuf = iterator.pixbuf();
                let texture = gtk::gdk::Texture::for_pixbuf(&pixbuf);
                texture.snapshot(snapshot, width, height);
            }
        }

        fn intrinsic_width(&self) -> i32 {
            let animation = self.animation.borrow();
            if let Some(ref animation) = *animation {
                return animation.width();
            }
            0
        }

        fn intrinsic_height(&self) -> i32 {
            let animation = self.animation.borrow();
            if let Some(ref animation) = *animation {
                return animation.height();
            }
            0
        }
    }
}
