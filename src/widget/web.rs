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
        Self::with_uri(&uri)
    }

    fn with_uri(uri: &str) -> Self {
        Object::builder().property("uri", uri).build()
    }
}

impl RendererWidget for WebWidget {
    fn mirror(&self) -> gtk::Box {
        let widget = gtk::Box::builder().build();
        let paintable = gtk::WidgetPaintable::new(Some(&self.webview()));
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

    fn play(&self) {}

    fn pause(&self) {}

    fn stop(&self) {}

    fn set_volume(&self, _volume: f64) {}

    fn set_mute(&self, _mute: bool) {}

    fn set_content_fit(&self, _fit: gtk::ContentFit) {}
}

mod imp {
    use super::*;

    use std::cell::RefCell;

    use glib::Properties;
    use gtk::subclass::prelude::*;
    use log::debug;
    use webkit::{prelude::*, WebView};

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::WebWidget)]
    pub struct WebWidget {
        #[property(get, set)]
        uri: RefCell<String>,
        #[property(get)]
        webview: RefCell<WebView>,
    }

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
            let webview = WebView::builder().build();
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
