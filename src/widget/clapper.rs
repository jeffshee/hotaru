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

use glib::Object;
use gtk::{gio, glib, prelude::*, subclass::prelude::*};
use tracing::info;

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
    fn mirror(&self, enable_graphics_offload: bool, content_fit: gtk::ContentFit) -> gtk::Box {
        let widget = gtk::Box::builder().build();
        let paintable = self.paintable().unwrap();
        let picture = gtk::Picture::builder()
            .paintable(&paintable)
            .hexpand(true)
            .vexpand(true)
            .content_fit(content_fit)
            .build();
        self.picture()
            .bind_property("content-fit", &picture, "content-fit")
            .build();

        #[cfg(feature = "gtk_v4_14")]
        if enable_graphics_offload {
            let offload = gtk::GraphicsOffload::new(Some(&picture));
            offload.set_enabled(gtk::GraphicsOffloadEnabled::Enabled);
            widget.append(&offload);
        } else {
            widget.append(&picture);
        }
        #[cfg(not(feature = "gtk_v4_14"))]
        {
            let _ = enable_graphics_offload;
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

    fn set_volume(&self, volume: f64) {
        self.player().set_volume(volume);
    }

    fn set_mute(&self, mute: bool) {
        self.player().set_mute(mute);
    }

    fn set_content_fit(&self, fit: gtk::ContentFit) {
        self.imp().content_fit.set(Some(fit));
        self.picture().set_content_fit(fit);
    }
}

mod imp {
    use super::*;

    use std::cell::{Cell, RefCell};

    use glib::Properties;
    use gtk::gdk;
    use tracing::{debug, error, warn};

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
        pub(super) content_fit: Cell<Option<gtk::ContentFit>>,
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

            info!("Using Clapper for video rendering");
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

            // Unlike GstGtk4 which owns its own Picture, Clapper's Picture
            // is managed by clappersink and may have its content-fit reset
            // during async pipeline state transitions. Re-apply on every
            // state change to ensure the user's setting persists.
            adapter.connect_closure(
                "state-changed",
                false,
                glib::closure_local!(
                    #[weak]
                    obj,
                    move |_adapter: gst_play::PlaySignalAdapter, playstate: gst_play::PlayState| {
                        debug!("{}", playstate);
                        let imp = obj.imp();
                        if let Some(fit) = imp.content_fit.get() {
                            imp.picture.borrow().set_content_fit(fit);
                        }
                    }
                ),
            );

            adapter.connect_warning(move |_adapter, error, _structure| {
                warn!("{}", error);
            });

            adapter.connect_error(move |_adapter, error, _structure| {
                error!("{}", error);
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
