use std::collections::HashMap;

use anyhow::Result;
use gtk::gio::ListModel;
use gtk::prelude::*;
use gtk::{gio, glib};
use hotaru::application::HotaruApplication;
use hotaru::gst_utils;
use hotaru::layout::DisplayConfiguration::{MirroredDisplay, PrimarySource};
use hotaru::layout::SourceIdentifier::{Filepath, Uri};
use hotaru::layout::{finalize_layout, DefaultLayout, Layout};
use hotaru::monitor::{MonitorInfo, MonitorListModelExt as _, MonitorTracker};
use hotaru::widget::{ClapperWidget, RendererWidget};
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
            let monitors: Vec<MonitorInfo> = list.try_to_monitor_info_vec().unwrap();
            println!("Monitor changed: {:?}", monitors);
            app_clone.windows().into_iter().for_each(|w| w.close());
            build_ui(&app_clone);
        }),
    );

    app.connect_activate(build_ui);

    Ok(app.run())
}

fn build_ui(app: &HotaruApplication) {
    let json = r#"{
        "source": {
            "filepath": "./test.webm",
            "type": "video"
        }
    }"#;
    let default_layout: DefaultLayout = serde_json::from_str(json).unwrap();
    let layout = Layout::Default(default_layout);
    let monitors = MonitorTracker::monitors()
        .unwrap()
        .try_to_monitor_info_vec()
        .unwrap();
    let monitors_clone = monitors.clone();

    let layout = finalize_layout(monitors_clone, layout).unwrap();

    let window_type = WindowType::X11Desktop;
    let mut source_widgets = HashMap::new();

    let monitors_clone = monitors.clone();

    // First pass: process all primary sources
    layout
        .0
        .iter()
        .filter(|c| matches!(c, PrimarySource { .. }))
        .for_each(|display_config| {
            if let PrimarySource {
                monitor,
                source,
                r#type,
            } = display_config
            {
                let monitor = monitors_clone
                    .iter()
                    .find(|m| m.connector == monitor.to_owned())
                    .unwrap();
                let geometry = monitor.geometry;
                let window = HotaruApplicationWindow::new(app, &window_type);

                window.set_size_request(geometry.width(), geometry.height());
                window.set_position(Position {
                    x: geometry.x(),
                    y: geometry.y(),
                });

                let widget = match source {
                    Filepath { filepath } => match r#type {
                        hotaru::layout::SourceType::Video => ClapperWidget::with_filepath(filepath),
                        hotaru::layout::SourceType::Web => todo!(),
                    },
                    Uri { uri } => match r#type {
                        hotaru::layout::SourceType::Video => ClapperWidget::with_uri(uri),
                        hotaru::layout::SourceType::Web => todo!(),
                    },
                };

                window.set_child(Some(&widget));
                window.present();
                widget.play();
                source_widgets.insert(monitor.connector.clone(), widget);
            }
        });

    let monitors_clone = monitors.clone();

    // Second pass: process mirrored displays after primary sources are created
    layout
        .0
        .iter()
        .filter(|c| matches!(c, MirroredDisplay { .. }))
        .for_each(|display_config| {
            if let MirroredDisplay { monitor, mirror_of } = display_config {
                if let Some(source_widget) = source_widgets.get(mirror_of) {
                    let monitor = monitors_clone
                        .iter()
                        .find(|m| m.connector == monitor.to_owned())
                        .unwrap();
                    let geometry = monitor.geometry;
                    let mirror_window = HotaruApplicationWindow::new(app, &window_type);

                    mirror_window.set_size_request(geometry.width(), geometry.height());
                    mirror_window.set_position(Position {
                        x: geometry.x(),
                        y: geometry.y(),
                    });
                    mirror_window.set_title(Some(&format!("Mirror of {}", mirror_of)));

                    let mirrored_widget = source_widget.mirror();
                    mirror_window.set_child(Some(&mirrored_widget));
                    mirror_window.present();
                }
            }
        });
}
