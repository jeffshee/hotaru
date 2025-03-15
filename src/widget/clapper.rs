/* clapper.rs
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

use glib::Object;
use gtk::prelude::*;
use gtk::{gio, glib};

use super::{RendererWidget, RendererWidgetBuilder};

glib::wrapper! {
    pub struct ClapperWidget(ObjectSubclass<imp::ClapperWidget>)
        @extends gtk::Box, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RendererWidgetBuilder for ClapperWidget {
    fn with_filepath(filepath: &str) -> Self {
        let uri = gio::File::for_path(filepath).uri();
        Self::with_uri(&uri)
    }

    fn with_uri(uri: &str) -> Self {
        Object::builder().property("uri", uri).build()
    }
}

impl RendererWidget for ClapperWidget {
    fn mirror(&self) -> gtk::Box {
        let widget = gtk::Box::builder().build();
        let paintable = self.paintable().unwrap();
        let picture = gtk::Picture::builder()
            .paintable(&paintable)
            .hexpand(true)
            .vexpand(true)
            .build();
        #[cfg(feature = "gtk_v4_14")]
        {
            let offload = gtk::GraphicsOffload::new(Some(&picture));
            offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
            widget.append(&offload);
        }
        #[cfg(not(feature = "gtk_v4_14"))]
        {
            widget.append(&picture);
        }
        widget
    }

    fn play(&self) {
        self.player().play();
    }

    fn pause(&self) {
        self.player().pause();
    }

    fn stop(&self) {
        self.player().stop();
    }
}

mod imp {
    use super::*;
    use glib::Properties;
    use gtk::{gdk, subclass::prelude::*};
    use std::cell::RefCell;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::ClapperWidget)]
    pub struct ClapperWidget {
        #[property(get, set)]
        uri: RefCell<String>,
        sink: RefCell<Option<gst::Element>>,
        renderer: RefCell<Option<gst_play::PlayVideoOverlayVideoRenderer>>,
        #[property(get)]
        player: RefCell<gst_play::Play>,
        #[property(get)]
        adapter: RefCell<Option<gst_play::PlaySignalAdapter>>,
        #[property(get)]
        paintable: RefCell<Option<gdk::Paintable>>,
        #[property(get)]
        picture: RefCell<gtk::Picture>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ClapperWidget {
        const NAME: &'static str = "ClapperWidget";
        type Type = super::ClapperWidget;
        type ParentType = gtk::Box;
    }

    #[glib::derived_properties]
    impl ObjectImpl for ClapperWidget {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();
            let sink = gst::ElementFactory::make("clappersink").build().unwrap();
            let renderer;
            #[cfg(feature = "gst_v1_24")]
            {
                if let Ok(glsink) = gst::ElementFactory::make("glsinkbin").build() {
                    glsink.set_property("sink", &sink);
                    renderer = gst_play::PlayVideoOverlayVideoRenderer::with_sink(&glsink);
                } else {
                    renderer = gst_play::PlayVideoOverlayVideoRenderer::with_sink(&sink);
                }
            }
            #[cfg(not(feature = "gst_v1_24"))]
            {
                renderer = gst_play::PlayVideoOverlayVideoRenderer::with_sink(&sink);
            }
            let player = gst_play::Play::new(Some(renderer.clone()));
            let adapter = gst_play::PlaySignalAdapter::new(&player);
            let picture = sink.property::<gtk::Picture>("widget");
            let paintable = picture.paintable().unwrap();
            picture.set_hexpand(true);
            picture.set_vexpand(true);

            obj.append(&picture);

            obj.bind_property("uri", &player, "uri")
                .bidirectional()
                .build();

            adapter.connect_end_of_stream(move |adapter| {
                adapter.play().seek(gst::ClockTime::from_seconds(0));
            });

            adapter.connect_state_changed(move |_adapter, playstate| {
                println!("{}", playstate);
            });

            adapter.connect_warning(move |_adapter, error, _structure| {
                eprintln!("{}", error);
            });

            adapter.connect_error(move |_adapter, error, _structure| {
                eprintln!("{}", error);
            });

            self.sink.replace(Some(sink));
            self.renderer.replace(Some(renderer));
            self.player.replace(player);
            self.adapter.replace(Some(adapter));
            self.paintable.replace(Some(paintable));
            self.picture.replace(picture);
        }
    }

    impl WidgetImpl for ClapperWidget {}

    impl BoxImpl for ClapperWidget {}
}
