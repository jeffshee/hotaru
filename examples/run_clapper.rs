use gtk::prelude::*;
use gtk::{gio, glib};
use hotaru::application::HotaruApplication;
use hotaru::gst_utils;
use hotaru::widget::{ClapperWidget, RendererWidget};
use hotaru::window::HotaruApplicationWindow;

fn main() -> glib::ExitCode {
    gst::init().unwrap();
    gtk::init().unwrap();
    gst_utils::setup_gst();

    let app = HotaruApplication::new(
        "io.github.jeffshee.Hotaru",
        &gio::ApplicationFlags::HANDLES_COMMAND_LINE,
    );
    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &HotaruApplication) {
    let main_window = HotaruApplicationWindow::new(app);

    let widget = ClapperWidget::with_filepath("./test.webm");
    main_window.set_child(Some(&widget));
    main_window.present();

    let window_mirror = HotaruApplicationWindow::new(app);
    window_mirror.set_title(Some(
        format!("{} <<mirror>>", window_mirror.title().unwrap()).as_str(),
    ));
    let widget_clone = widget.mirror();
    window_mirror.set_child(Some(&widget_clone));
    window_mirror.present();

    widget.play();
}
