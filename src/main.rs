/* main.rs
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

mod application;
mod clapper_widget;
mod config;
mod gif_paintable;
mod gst_utils;
mod video_widget;
mod window;

use config::{GETTEXT_PACKAGE, LOCALEDIR, PKGDATADIR};
use gettextrs::{bind_textdomain_codeset, bindtextdomain, textdomain};
use gtk::prelude::*;
use gtk::Picture;
use gtk::{gio, glib};
use webkit::{prelude::*, WebView};

use application::HotaruApplication;
use clapper_widget::ClapperWidget;
use gif_paintable::GifPaintable;
use video_widget::VideoWidget;
use window::HotaruApplicationWindow;

fn main() -> glib::ExitCode {
    gst::init().unwrap();
    gtk::init().unwrap();
    gstgtk4::plugin_register_static().expect("Failed to register gstgtk4 plugin");
    gst_utils::setup_gst();

    // Set up gettext translations
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).expect("Unable to bind the text domain");
    bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8")
        .expect("Unable to set the text domain encoding");
    textdomain(GETTEXT_PACKAGE).expect("Unable to switch to the text domain");

    // Load resources
    let resources = gio::Resource::load(PKGDATADIR.to_owned() + "/hotaru.gresource")
        .expect("Could not load resources");
    gio::resources_register(&resources);

    let app = HotaruApplication::new(
        "io.github.jeffshee.Hotaru",
        &gio::ApplicationFlags::HANDLES_COMMAND_LINE,
    );
    // app.connect_activate(build_ui_gif);
    // app.connect_activate(build_ui_video);
    app.connect_activate(build_ui_clapper);
    // app.connect_activate(build_ui_web);
    app.run()
}

#[allow(dead_code)]
fn build_ui_web(app: &HotaruApplication) {
    let window = HotaruApplicationWindow::new(app);
    let webview = WebView::builder().build();
    webview.load_uri("https://jeffshee.github.io/herta-wallpaper/");
    window.set_child(Some(&webview));
    window.present();

    let window_clone = HotaruApplicationWindow::new(app);
    window_clone.set_title(Some(
        format!("{}#2", window_clone.title().unwrap()).as_str(),
    ));
    let widget = gtk::Box::builder().build();
    let paintable = gtk::WidgetPaintable::new(Some(&webview));
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
    window_clone.set_child(Some(&widget));
    window_clone.present();
}

#[allow(dead_code)]
fn build_ui_gif(app: &HotaruApplication) {
    let window = HotaruApplicationWindow::new(app);

    let gif = GifPaintable::with_filepath("./test.gif");
    let picture = Picture::for_paintable(&gif);
    window.set_child(Some(&picture));
    window.present();

    gif.animate();
}

#[allow(dead_code)]
fn build_ui_video(app: &HotaruApplication) {
    let window = HotaruApplicationWindow::new(app);

    let video_widget = VideoWidget::with_filepath("./test.webm");
    window.set_child(Some(&video_widget));
    window.present();

    let window_clone = HotaruApplicationWindow::new(app);
    window_clone.set_title(Some(
        format!("{}#2", window_clone.title().unwrap()).as_str(),
    ));
    let widget_clone = video_widget.widget_clone();
    window_clone.set_child(Some(&widget_clone));
    window_clone.present();

    video_widget.player().play();
}

#[allow(dead_code)]
fn build_ui_clapper(app: &HotaruApplication) {
    let window = HotaruApplicationWindow::new(app);

    let clapper_widget = ClapperWidget::with_filepath("./test.webm");
    window.set_child(Some(&clapper_widget));
    window.present();

    let window_clone = HotaruApplicationWindow::new(app);
    window_clone.set_title(Some(
        format!("{}#2", window_clone.title().unwrap()).as_str(),
    ));
    let widget_clone = clapper_widget.widget_clone();
    window_clone.set_child(Some(&widget_clone));
    window_clone.present();

    clapper_widget.player().play();
}
