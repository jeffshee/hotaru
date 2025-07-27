use std::{env, path::PathBuf};

use clap::{ArgAction, Parser};

use hotaru::prelude::*;

#[derive(Debug, Parser)]
#[command(author, version, about)]
pub struct Cli {
    /// Launch mode
    #[arg(short = 'l', long = "launch-mode", default_value_t)]
    pub launch_mode: LaunchMode,

    /// Path to config JSON file
    #[arg(short = 'c', long = "config", value_name = "FILE")]
    pub config_file: PathBuf,

    /// Log level
    #[clap(long, default_value = "info")]
    pub log_level: String,

    /// Use Clapper for video playback
    #[arg(
        long,
        default_value_t = true,
        action = ArgAction::Set
    )]
    use_clapper: bool,

    /// Enable VA decoders for improved performance on Intel/AMD Wayland setups
    #[arg(long, default_value_t = false)]
    enable_va: bool,

    /// Enable stateless NVIDIA decoders
    #[arg(long, default_value_t = false)]
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
