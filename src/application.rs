/* application.rs
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

glib::wrapper! {
    pub struct HotaruApplication(ObjectSubclass<imp::HotaruApplication>)
        @extends gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl HotaruApplication {
    pub fn new(application_id: &str, flags: &gio::ApplicationFlags) -> Self {
        Object::builder()
            .property("application_id", application_id)
            .property("flags", flags)
            .build()
    }
}

mod imp {
    use super::*;
    use std::cell::Cell;
    use std::env;
    use std::process::{exit, Command};

    use gtk::gdk::Display;
    use gtk::glib::{self, Char, Properties, Type};
    use gtk::subclass::prelude::*;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::HotaruApplication)]
    pub struct HotaruApplication {
        #[property(get, set)]
        use_x11_desktop: Cell<bool>,
        #[property(get, set)]
        use_wayland_layer_shell: Cell<bool>,
        #[property(get, set)]
        use_gnome_hanabi_ext: Cell<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HotaruApplication {
        const NAME: &'static str = "HotaruApplication";
        type Type = super::HotaruApplication;
        type ParentType = gtk::Application;
    }

    #[glib::derived_properties]
    impl ObjectImpl for HotaruApplication {
        fn constructed(&self) {
            self.parent_constructed();

            let app = self.obj();
            app.add_main_option(
                "x11-desktop",
                Char::try_from('x').unwrap(),
                glib::OptionFlags::NONE,
                glib::OptionArg::None,
                "Use X11 Extended Window Manager Hints",
                None,
            );
            app.add_main_option(
                "wayland-layer-shell",
                Char::try_from('w').unwrap(),
                glib::OptionFlags::NONE,
                glib::OptionArg::None,
                "Use Wayland Layer Shell Protocol",
                None,
            );
            app.add_main_option(
                "gnome-hanabi-ext",
                Char::try_from('g').unwrap(),
                glib::OptionFlags::NONE,
                glib::OptionArg::None,
                "Use Gnome Hanabi Extension",
                None,
            );
        }
    }

    impl ApplicationImpl for HotaruApplication {
        fn command_line(&self, command_line: &gtk::gio::ApplicationCommandLine) -> glib::ExitCode {
            let options = command_line.options_dict();

            let use_x11_desktop = options.contains("x11-desktop");
            let use_wayland_layer_shell = options.contains("wayland-layer-shell");
            let use_gnome_hanabi_ext = options.contains("gnome-hanabi-ext");

            if (use_x11_desktop as u8 + use_wayland_layer_shell as u8 + use_gnome_hanabi_ext as u8)
                > 1
            {
                eprintln!("Specify only one launch mode: `--x11-desktop`, `--wayland-layer-shell` or `--gnome-hanabi-ext`");
                return glib::ExitCode::from(1);
            }

            let app = self.obj();
            app.set_use_x11_desktop(use_x11_desktop);
            app.set_use_wayland_layer_shell(use_wayland_layer_shell);
            app.set_use_gnome_hanabi_ext(use_gnome_hanabi_ext);

            if use_x11_desktop {
                println!("Using X11 Extended Window Manager Hints");
                check_x11();
            } else if use_wayland_layer_shell {
                println!("Using Wayland Layer Shell Protocol");
                panic!("Not implemented yet");
            } else if use_gnome_hanabi_ext {
                println!("Using Gnome Hanabi Extension");
                panic!("Not implemented yet");
            } else {
                println!("Using Standalone Mode");
            }

            app.activate();
            glib::ExitCode::from(0)
        }
    }

    impl GtkApplicationImpl for HotaruApplication {}

    fn check_x11() {
        let display = Display::default().expect("Failed to get default display");
        let is_x11 = display
            .type_()
            .is_a(Type::from_name("GdkX11Display").unwrap());
        if !is_x11 {
            // Fallback to XWayland
            env::set_var("GDK_BACKEND", "x11");
            let args: Vec<String> = env::args().collect();
            Command::new(&args[0])
                .args(&args[1..])
                .spawn()
                .expect("Failed to fallback to XWayland");
            exit(0);
        }
    }
}
