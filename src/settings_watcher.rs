// Copyright (C) 2026  Jeff Shee
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: GPL-3.0-or-later

use std::cell::RefCell;
use std::rc::Rc;

use gtk::gio;
use gtk::prelude::*;
use tracing::info;

use crate::constant::APPLICATION_ID;
use crate::widget::{Renderer, RendererWidget};

/// Watches the Hotaru GSettings schema and applies changes to active renderers.
pub struct SettingsWatcher {
    settings: gio::Settings,
}

impl Default for SettingsWatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsWatcher {
    pub fn new() -> Self {
        let settings = gio::Settings::new(APPLICATION_ID);
        Self { settings }
    }

    pub fn settings(&self) -> &gio::Settings {
        &self.settings
    }

    pub fn is_use_clapper(&self) -> bool {
        self.settings.boolean("use-clapper")
    }

    pub fn is_enable_graphics_offload(&self) -> bool {
        self.settings.boolean("enable-graphics-offload")
    }

    /// Read the current volume as a 0.0-1.0 float from the 0-100 int setting.
    pub fn volume(&self) -> f64 {
        self.settings.int("volume") as f64 / 100.0
    }

    pub fn is_mute(&self) -> bool {
        self.settings.boolean("mute")
    }

    pub fn content_fit(&self) -> gtk::ContentFit {
        content_fit_from_int(self.settings.int("content-fit"))
    }

    /// Connect GSettings change signals to update active renderers at runtime.
    /// The `renderers` Rc is shared with application state and updated when
    /// wallpapers are applied or disabled.
    pub fn connect_runtime_settings(&self, renderers: Rc<RefCell<Vec<Renderer>>>) {
        let renderers_clone = renderers.clone();
        self.settings
            .connect_changed(Some("volume"), move |settings, _key| {
                let volume = settings.int("volume") as f64 / 100.0;
                info!("Volume changed to: {:.0}%", volume * 100.0);
                for renderer in renderers_clone.borrow().iter() {
                    renderer.set_volume(volume);
                }
            });

        let renderers_clone = renderers.clone();
        self.settings
            .connect_changed(Some("mute"), move |settings, _key| {
                let mute = settings.boolean("mute");
                info!("Mute changed to: {}", mute);
                for renderer in renderers_clone.borrow().iter() {
                    renderer.set_mute(mute);
                }
            });

        let renderers_clone = renderers.clone();
        self.settings
            .connect_changed(Some("content-fit"), move |settings, _key| {
                let fit = content_fit_from_int(settings.int("content-fit"));
                info!("Content fit changed to: {:?}", fit);
                for renderer in renderers_clone.borrow().iter() {
                    renderer.set_content_fit(fit);
                }
            });
    }

    // --- Last applied wallpaper persistence ---

    pub fn last_wallpaper_config(&self) -> String {
        self.settings.string("last-wallpaper-config").to_string()
    }

    pub fn set_last_wallpaper_config(&self, value: &str) {
        self.settings
            .set_string("last-wallpaper-config", value)
            .ok();
    }

    pub fn last_launch_mode(&self) -> String {
        self.settings.string("last-launch-mode").to_string()
    }

    pub fn set_last_launch_mode(&self, value: &str) {
        self.settings.set_string("last-launch-mode", value).ok();
    }
}

fn content_fit_from_int(value: i32) -> gtk::ContentFit {
    match value {
        0 => gtk::ContentFit::Fill,
        1 => gtk::ContentFit::Contain,
        2 => gtk::ContentFit::Cover,
        3 => gtk::ContentFit::ScaleDown,
        _ => gtk::ContentFit::Contain,
    }
}
