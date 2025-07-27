use std::{env, path::PathBuf};

use clap::{ArgAction, Parser};

use hotaru::prelude::*;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    #[arg(
        short = 'l',
        long = "launch-mode",
        default_value_t,
        help = "Launch mode"
    )]
    pub launch_mode: LaunchMode,

    #[arg(
        short = 'c',
        long = "config",
        value_name = "FILE",
        help = "Path to the wallpaper config JSON file"
    )]
    pub config_file: PathBuf,

    #[arg(
        long,
        default_value = "info",
        help = "Logging level following RUST_LOG format"
    )]
    pub log_level: String,

    #[arg(
        long,
        default_value_t = true,
        action = ArgAction::Set,
        help = "Use Clapper for video playback",
        long_help = "Use Clapper as the video playback backend. \
                     Set to false to use the GTK4 Plugin from GStreamer instead.",
    )]
    use_clapper: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Enable VA decoders for improved performance on Intel/AMD Wayland setups (experimental; default: false)"
    )]
    enable_va: bool,

    #[arg(
        long,
        default_value_t = false,
        help = "Enable stateless NVIDIA decoders, which may improve NVIDIA hardware acceleration (experimental; default: false)"
    )]
    enable_nvsl: bool,
}

impl Cli {
    pub fn is_use_clapper(&self) -> bool {
        if self.use_clapper {
            true
        } else {
            env::var("USE_CLAPPER").is_ok_and(|v| v == "true")
        }
    }

    pub fn is_enable_va(&self) -> bool {
        if self.enable_va {
            true
        } else {
            env::var("ENABLE_VA").is_ok_and(|v| v == "true")
        }
    }

    pub fn is_enable_nvsl(&self) -> bool {
        if self.enable_nvsl {
            true
        } else {
            env::var("ENABLE_NVSL").is_ok_and(|v| v == "true")
        }
    }
}
