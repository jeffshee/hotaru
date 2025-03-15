use std::collections::HashMap;

use gtk::gdk::prelude::MonitorExt as _;
use gtk::gio::ListModel;
use gtk::prelude::*;
use gtk::{gio, glib};
use hotaru::application::HotaruApplication;
use hotaru::gst_utils;
use hotaru::layout::MonitorConfig::{Mirror, Source};
use hotaru::layout::SourceIdentifier::{Filepath, Uri};
use hotaru::layout::{DefaultLayout, Layout};
use hotaru::monitor::{MonitorExt, MonitorInfo, MonitorListModelExt as _, MonitorManager};
use hotaru::widget::{Renderer, RendererWidget};
use hotaru::window::{HotaruApplicationWindow, Position, WindowType};

const DEFAULT_LAYOUT_JSON: &str = r#"{
    "source": {
        "filepath": "./test.webm",
        "type": "video"
    }
}"#;

// const DEFAULT_LAYOUT_JSON: &str = r#"{
//     "source": {
//         "uri": "https://jeffshee.github.io/herta-wallpaper/",
//         "type": "web"
//     }
// }"#;

fn main() -> glib::ExitCode {
    gst::init().unwrap();
    gtk::init().unwrap();
    gst_utils::setup_gst();

    let app = HotaruApplication::new(
        "io.github.jeffshee.Hotaru",
        &gio::ApplicationFlags::HANDLES_COMMAND_LINE,
    );

    let app_clone = app.clone();
    let monitor_tracker = MonitorManager::new();
    monitor_tracker.connect_closure(
        "monitor-changed",
        false,
        glib::closure_local!(move |_monitor_tracker: MonitorManager, list: ListModel| {
            let monitors: Vec<MonitorInfo> = list.try_to_monitor_info_vec().unwrap();
            println!("Monitor changed: {:?}", monitors);
            app_clone.windows().into_iter().for_each(|w| w.close());
            build_ui(&app_clone);
        }),
    );

    app.connect_activate(build_ui);

    app.run()
}

fn build_ui(app: &HotaruApplication) {
    let window_type = WindowType::X11Desktop;
    let monitors = MonitorManager::monitors()
        .unwrap()
        .try_to_monitor_vec()
        .unwrap();

    let default_layout: DefaultLayout = serde_json::from_str(DEFAULT_LAYOUT_JSON).unwrap();
    let layout = Layout::Default(default_layout);
    let final_layout = layout.finalize(&monitors).unwrap();

    let mut source_renderers = HashMap::new();

    // First pass: process source
    final_layout
        .configs
        .iter()
        .filter(|c| matches!(c, Source { .. }))
        .for_each(|display_config| {
            if let Source {
                monitor,
                source,
                r#type,
            } = display_config
            {
                let monitor = monitors.iter().find(|m| m.is_connector(monitor)).unwrap();
                let geometry = monitor.geometry();

                // Create window
                let window = HotaruApplicationWindow::new(app, &window_type);
                window.set_size_request(geometry.width(), geometry.height());
                window.set_position(Position {
                    x: geometry.x(),
                    y: geometry.y(),
                });

                let renderer = match source {
                    Filepath { filepath } => Renderer::with_filepath(filepath, r#type),
                    Uri { uri } => Renderer::with_uri(uri, r#type),
                };
                let widget = renderer.as_widget();

                window.set_child(Some(widget));
                window.present();
                renderer.play();

                source_renderers.insert(monitor.connector().unwrap().to_string(), renderer);
            }
        });

    // Second pass: process mirror
    final_layout
        .configs
        .iter()
        .filter(|c| matches!(c, Mirror { .. }))
        .for_each(|display_config| {
            if let Mirror { monitor, mirror_of } = display_config {
                if let Some(source_widget) = source_renderers.get(mirror_of) {
                    let monitor = monitors.iter().find(|m| m.is_connector(monitor)).unwrap();
                    let geometry = monitor.geometry();

                    // Create window
                    let window = HotaruApplicationWindow::new(app, &window_type);
                    window.set_size_request(geometry.width(), geometry.height());
                    window.set_position(Position {
                        x: geometry.x(),
                        y: geometry.y(),
                    });

                    let widget = source_widget.mirror();
                    window.set_child(Some(&widget));
                    window.present();
                }
            }
        });
}
