use std::collections::HashMap;

use anyhow::Result;
use gtk::gio::ListModel;
use gtk::prelude::*;
use gtk::{gio, glib};
use hotaru::application::HotaruApplication;
use hotaru::gst_utils;
use hotaru::layout_v2::{convert_to_window_layout, LiveWallpaperConfig, WallpaperType, SourceType, WindowInfo};
use hotaru::monitor_v2::{MonitorListModelExt as _, MonitorTracker};
use hotaru::widget::{GstGtk4Widget, RendererWidget, WebWidget};
use hotaru::window::{HotaruApplicationWindow, Position, WindowType};

fn main() -> Result<glib::ExitCode> {
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

    Ok(app.run())
}

fn build_ui(app: &HotaruApplication) {
    let json = r#"
{
  "mode": "wallpaper_per_monitor",
  "monitors": [
    {
      "monitor": "DP-5",
      "wallpaper_type": "video",
      "source_type": "filepath",
      "source": "./test.webm"
    },
    {
      "monitor": "DP-4",
      "wallpaper_type": "video",
      "source_type": "filepath",
      "source": "./test.mp4"
    }
  ]
}
"#;

//     let json = r#"
// {
//   "mode": "stretch_single_wallpaper",
//   "monitors": [
//     {
//       "monitor": "STRETCH",
//       "wallpaper_type": "video",
//       "source_type": "filepath",
//       "source": "./test.webm"
//     }
//   ]
// }
// "#;

    let config: LiveWallpaperConfig = serde_json::from_str(json).unwrap();
    let monitor_map = MonitorTracker::monitors()
        .unwrap()
        .try_to_monitor_map()
        .unwrap();
    let layout = convert_to_window_layout(&config, &monitor_map);
    let window_type = WindowType::X11Desktop;
    let mut source_widgets = HashMap::new();

    layout.windows.iter().for_each(|window_info| {
        if let WindowInfo::Primary {
            monitor,
            window_x,
            window_y,
            window_width,
            window_height,
            window_title,
            wallpaper_type,
            source_type,
            source,
        } = window_info
        {
            let window = HotaruApplicationWindow::new(app, &window_type);
            window.set_position(Position{x: *window_x, y: *window_y});
            window.set_size_request(*window_width, *window_height);
            window.set_title(Some(&window_title));
            let widget = match wallpaper_type {
                WallpaperType::Video => match source_type {
                    SourceType::Filepath => GstGtk4Widget::with_filepath(&source),
                    SourceType::Uri => GstGtk4Widget::with_uri(&source) 
                }
                WallpaperType::Web => todo!()

                // WallpaperType::Web => match source_type {
                //     SourceType::Filepath => WebWidget::with_filepath(&soruce),
                //     SourceType::Uri => WebWidget::with_uri(&soruce),
                // },
            };

            window.set_child(Some(&widget));
            window.present();
            widget.play();
            source_widgets.insert(monitor.to_string(), widget );
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
            window.set_position(Position{x: *window_x, y: *window_y});
            window.set_size_request(*window_width, *window_height);
            window.set_title(Some(&window_title));
            if let Some(source_widget) = source_widgets.get(clone_source){
                let mirrored_widget = source_widget.mirror();
                window.set_child(Some(&mirrored_widget));
            }
            window.present();
        }
    });
}
