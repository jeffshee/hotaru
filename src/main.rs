mod cli;

use clap::Parser as _;
use gtk::{
    gio::{ApplicationFlags, ListModel},
    glib,
    prelude::*,
};
use log::{debug, info};

use hotaru::{
    application::HotaruApplication,
    constant::APPLICATION_ID,
    gst_utils,
    layout::LiveWallpaperConfig,
    monitor::{MonitorListModelExt as _, MonitorTracker},
};

use crate::cli::Cli;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let is_enable_va = cli.is_enable_va();
    let is_enable_nvsl = cli.is_enable_nvsl();
    let is_use_clapper = cli.is_use_clapper();

    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(&cli.log_level)).init();
    info!("Hotaru started with args: {:#?}", cli);

    let json = std::fs::read_to_string(cli.config_file.clone())?;
    let config: LiveWallpaperConfig = serde_json::from_str(&json)?;
    info!("Live wallpaper config loaded: {:#?}", config);

    gst::init().unwrap();
    gtk::init().unwrap();
    gst_utils::setup_gst(is_enable_va, is_enable_nvsl);

    let app = HotaruApplication::new(
        APPLICATION_ID,
        &ApplicationFlags::HANDLES_COMMAND_LINE,
        cli.launch_mode,
    );

    let app_clone = app.clone();
    let config_clone = config.clone();
    let monitor_tracker = MonitorTracker::new();
    monitor_tracker.connect_closure(
        "monitor-changed",
        false,
        glib::closure_local!(move |_monitor_tracker: MonitorTracker, list: ListModel| {
            let monitor_map = list.try_to_monitor_map().unwrap();
            debug!("monitor changed: {:?}", monitor_map);
            app_clone.windows().into_iter().for_each(|w| w.close());
            app_clone.build_ui(&config_clone, is_use_clapper);
        }),
    );

    app.connect_activate(move |app| app.build_ui(&config, is_use_clapper));
    app.run();

    Ok(())
}
