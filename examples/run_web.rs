use gtk::prelude::*;
use gtk::{gio, glib};
use hotaru::application::HotaruApplication;
use hotaru::widget::{RendererWidget, RendererWidgetBuilder, WebWidget};
use hotaru::window::{HotaruApplicationWindow, WindowType};

fn main() -> glib::ExitCode {
    gtk::init().unwrap();

    let app = HotaruApplication::new(
        "io.github.jeffshee.Hotaru",
        &gio::ApplicationFlags::HANDLES_COMMAND_LINE,
    );
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &HotaruApplication) {
    let window_type = WindowType::Standalone;
    let main_window = HotaruApplicationWindow::new(app, &window_type);

    let widget = WebWidget::with_uri("https://jeffshee.github.io/herta-wallpaper/");

    main_window.set_child(Some(&widget));
    main_window.present();

    let window_mirror = HotaruApplicationWindow::new(app, &window_type);
    window_mirror.set_title(Some(
        format!("{} <<mirror>>", window_mirror.title().unwrap_or_default()).as_str(),
    ));
    let widget_clone = widget.mirror();
    window_mirror.set_child(Some(&widget_clone));
    window_mirror.present();

    widget.play();
}
