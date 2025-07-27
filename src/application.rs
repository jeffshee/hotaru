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

use std::{collections::HashMap, str::FromStr as _};

use glib::Object;
use gtk::{gio, glib, prelude::*};
use log::info;

use crate::{
    layout::{convert_to_window_layout, LiveWallpaperConfig, WallpaperSource, WindowInfo},
    model::LaunchMode,
    monitor::{MonitorListModelExt as _, MonitorTracker},
    widget::{Renderer, RendererWidget},
    window::{HotaruApplicationWindow, Position},
};

glib::wrapper! {
    pub struct HotaruApplication(ObjectSubclass<imp::HotaruApplication>)
        @extends gtk::Application, gio::Application,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl HotaruApplication {
    pub fn new(
        application_id: &str,
        flags: &gio::ApplicationFlags,
        launch_mode: LaunchMode,
    ) -> Self {
        Object::builder()
            .property("application_id", application_id)
            .property("flags", flags)
            .property("launch_mode", launch_mode)
            .build()
    }

    pub fn build_ui(&self, config: &LiveWallpaperConfig, use_clapper: bool) {
        let launch_mode = LaunchMode::from_str(&self.launch_mode()).unwrap();
        let monitor_map = MonitorTracker::monitors()
            .unwrap()
            .try_to_monitor_map()
            .unwrap();
        info!("Monitor map: {:#?}", monitor_map);

        let layout = convert_to_window_layout(config, &monitor_map);
        info!("Window layout: {:#?}", layout);
        let mut primary_widgets = HashMap::new();

        layout.windows.iter().for_each(|window_info| {
            if let WindowInfo::Primary {
                monitor,
                window_x,
                window_y,
                window_width,
                window_height,
                window_title,
                wallpaper_type,
                wallpaper_source,
            } = window_info
            {
                let window = HotaruApplicationWindow::new(self, launch_mode);
                window.set_position(Position {
                    x: *window_x,
                    y: *window_y,
                });
                window.set_size_request(*window_width, *window_height);
                window.set_title(Some(window_title));
                let renderer = match wallpaper_source {
                    WallpaperSource::Filepath { filepath } => {
                        Renderer::with_filepath(filepath, wallpaper_type, use_clapper)
                    }
                    WallpaperSource::Uri { uri } => {
                        Renderer::with_uri(uri, wallpaper_type, use_clapper)
                    }
                };
                window.set_child(Some(renderer.widget()));
                window.present();
                renderer.play();
                primary_widgets.insert(monitor.to_string(), renderer);
            }
        });

        layout.windows.iter().for_each(|window_info| {
            if let WindowInfo::Clone {
                monitor: _,
                window_x,
                window_y,
                window_width,
                window_height,
                window_title,
                clone_source,
            } = window_info
            {
                let window = HotaruApplicationWindow::new(self, launch_mode);
                window.set_position(Position {
                    x: *window_x,
                    y: *window_y,
                });
                window.set_size_request(*window_width, *window_height);
                window.set_title(Some(window_title));
                if let Some(primary_widget) = primary_widgets.get(clone_source) {
                    let widget = primary_widget.mirror();
                    window.set_child(Some(&widget));
                }
                window.present();
            }
        });
    }
}

mod imp {
    use super::*;

    use std::{
        cell::RefCell,
        env,
        process::{exit, Command},
    };

    use gtk::{
        gdk::Display,
        glib::{self, Properties, Type},
        subclass::prelude::*,
    };

    use crate::constant::LAUNCH_MODE_X11_DESKTOP;

    #[derive(Properties, Default)]
    #[properties(wrapper_type = super::HotaruApplication)]
    pub struct HotaruApplication {
        #[property(get, construct_only)]
        launch_mode: RefCell<String>,
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

            let obj = self.obj();
            if obj.launch_mode() == LAUNCH_MODE_X11_DESKTOP {
                fallback_to_xwayland();
            }
        }
    }

    impl ApplicationImpl for HotaruApplication {
        fn command_line(&self, _command_line: &gio::ApplicationCommandLine) -> glib::ExitCode {
            // Just activate the application, we already handled the cli arguments with clap
            let app = self.obj();
            app.activate();
            glib::ExitCode::from(0)
        }
    }

    impl GtkApplicationImpl for HotaruApplication {}

    fn fallback_to_xwayland() {
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
