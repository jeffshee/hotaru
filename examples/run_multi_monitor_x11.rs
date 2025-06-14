use gtk::gio::ListModel;
use gtk::prelude::*;
use gtk::{gio, glib};
use hotaru::application::HotaruApplication;
use hotaru::gst_utils;
use hotaru::layout::{
    convert_to_window_layout, LiveWallpaperConfig, WallpaperSource, WindowInfo,
};
use hotaru::monitor::{MonitorListModelExt as _, MonitorTracker};
use hotaru::widget::{Renderer, RendererWidget};
use hotaru::window::{HotaruApplicationWindow, Position, WindowType};
use std::collections::HashMap;
use std::env;

fn main() -> glib::ExitCode {
    // env::set_var("VIDEO_USE_CLAPPER", "1");
    env::set_var("VIDEO_USE_CLAPPER", "0");

    gst::init().unwrap();
    gtk::init().unwrap();
    gst_utils::setup_gst();

    let app = HotaruApplication::new(
        "io.github.jeffshee.Hotaru",
        &gio::ApplicationFlags::HANDLES_COMMAND_LINE,
    );

    let app_clone = app.clone();
    let monitor_tracker = MonitorTracker::new();
    monitor_tracker.connect_closure(
        "monitor-changed",
        false,
        glib::closure_local!(move |_monitor_tracker: MonitorTracker, list: ListModel| {
            let monitor_map = list.try_to_monitor_map().unwrap();
            println!("Monitor changed: {:?}", monitor_map);
            app_clone.windows().into_iter().for_each(|w| w.close());
            build_ui(&app_clone);
        }),
    );

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &HotaruApplication) {
    let json = r#"
    {
        "mode": "wallpaper_per_monitor",
        "monitors": [
            {
                "monitor": "DP-5",
                "wallpaper_type": "video",
                "filepath": "./test.webm"
            },
            {
                "monitor": "DP-4",
                "wallpaper_type": "web",
                "uri": "https://jeffshee.github.io/herta-wallpaper/"
            }
        ]
    }
    "#;

    // let json = r#"
    // {
    //     "mode": "clone_single_wallpaper",
    //     "monitors": [
    //         {
    //             "monitor": "DP-5",
    //             "wallpaper_type": "video",
    //             "filepath": "./test.webm"
    //         },
    //         {
    //             "monitor": "DP-4"
    //         }
    //     ]
    // }
    // "#;

    // let json = r#"
    // {
    //     "mode": "stretch_single_wallpaper",
    //     "monitors": [
    //         {
    //             "monitor": "STRETCH",
    //             "wallpaper_type": "video",
    //             "filepath": "./test.webm"
    //         }
    //     ]
    // }
    // "#;

    let config: LiveWallpaperConfig = serde_json::from_str(json).unwrap();
    println!("{:#?}", config);

    let monitor_map = MonitorTracker::monitors()
        .unwrap()
        .try_to_monitor_map()
        .unwrap();
    println!("{:#?}", monitor_map);
    let layout = convert_to_window_layout(&config, &monitor_map);
    let window_type = WindowType::X11Desktop;
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
            let window = HotaruApplicationWindow::new(app, &window_type);
            window.set_position(Position {
                x: *window_x,
                y: *window_y,
            });
            window.set_size_request(*window_width, *window_height);
            window.set_title(Some(&window_title));
            let renderer = match wallpaper_source {
                WallpaperSource::Filepath { filepath } => {
                    Renderer::with_filepath(filepath, wallpaper_type)
                }
                WallpaperSource::Uri { uri } => Renderer::with_uri(uri, wallpaper_type),
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
            let window = HotaruApplicationWindow::new(app, &window_type);
            window.set_position(Position {
                x: *window_x,
                y: *window_y,
            });
            window.set_size_request(*window_width, *window_height);
            window.set_title(Some(&window_title));
            if let Some(primary_widget) = primary_widgets.get(clone_source) {
                let widget = primary_widget.mirror();
                window.set_child(Some(&widget));
            }
            window.present();
        }
    });
}
