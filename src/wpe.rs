// Copyright (C) 2026 Jeff Shee <jeffshee8969@gmail.com>
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

//! Wallpaper Engine package resolution.
//!
//! A WPE workshop item is a directory with a `project.json` describing its
//! `type` (scene / video / web) and entry `file`. This module resolves a
//! config source (a directory path or a workshop id) to that directory and
//! reads the project descriptor, so the renderer layer can delegate scene
//! packages to linux-wallpaperengine and video/web packages to hotaru's own
//! renderers.

use std::path::PathBuf;
use std::{env, fs};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::constants::WPE_WORKSHOP_APP_ID;
use crate::model::WallpaperSource;

/// Environment override pointing directly at the workshop content directory
/// (the one containing `<workshop-id>` subdirectories), e.g.
/// `.../steamapps/workshop/content/431960`.
const WORKSHOP_ENV: &str = "HOTARU_WPE_WORKSHOP";

/// The renderer a WPE package maps to, from its `project.json` `type`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WpeType {
    Scene,
    Video,
    Web,
}

#[derive(Deserialize)]
struct ProjectJson {
    #[serde(rename = "type")]
    kind: String,
    file: Option<String>,
    #[serde(default)]
    general: General,
}

#[derive(Deserialize, Default)]
struct General {
    #[serde(default)]
    properties: serde_json::Map<String, serde_json::Value>,
}

/// A resolved Wallpaper Engine package.
pub struct WpePackage {
    /// The package directory (contains `project.json`).
    pub dir: PathBuf,
    pub kind: WpeType,
    /// Entry file relative to `dir` (the video file or `index.html`); unused
    /// for scene packages, which are handed to the engine as a directory.
    file: Option<String>,
    /// Raw `general.properties` from project.json (used for web packages).
    properties: serde_json::Map<String, serde_json::Value>,
}

impl WpePackage {
    /// Resolve a config source to a package and read its `project.json`.
    pub fn resolve(source: &WallpaperSource) -> Result<Self> {
        let dir = match source {
            WallpaperSource::Filepath { filepath } => PathBuf::from(filepath),
            WallpaperSource::WorkshopId { workshop_id } => resolve_workshop_id(workshop_id)?,
            WallpaperSource::Uri { .. } => {
                bail!("a wpe wallpaper cannot be specified as a URI (use filepath or workshop_id)")
            }
        };
        Self::from_dir(dir)
    }

    fn from_dir(dir: PathBuf) -> Result<Self> {
        let project = dir.join("project.json");
        let data = fs::read_to_string(&project)
            .with_context(|| format!("reading {}", project.display()))?;
        let parsed: ProjectJson = serde_json::from_str(&data)
            .with_context(|| format!("parsing {}", project.display()))?;

        // Wallpaper Engine capitalizes the type inconsistently ("scene" vs
        // "Scene"), so match case-insensitively.
        let kind = match parsed.kind.to_ascii_lowercase().as_str() {
            "scene" => WpeType::Scene,
            "video" => WpeType::Video,
            "web" => WpeType::Web,
            _ => bail!(
                "unsupported Wallpaper Engine type {:?} in {}",
                parsed.kind,
                project.display()
            ),
        };

        Ok(Self {
            dir,
            kind,
            file: parsed.file,
            properties: parsed.general.properties,
        })
    }

    /// The absolute entry file (`dir`/`file`), for video and web packages.
    pub fn entry(&self) -> Result<PathBuf> {
        let file = self
            .file
            .as_ref()
            .context("project.json has no \"file\" entry")?;
        Ok(self.dir.join(file))
    }

    /// The default user properties as the argument Wallpaper Engine passes to
    /// `wallpaperPropertyListener.applyUserProperties` — `{name: {value: …}}`
    /// built from each property's default `value` in project.json. Serialized
    /// as a JSON string ready to embed in injected JavaScript.
    pub fn user_properties_json(&self) -> String {
        let mut out = serde_json::Map::new();
        for (name, def) in &self.properties {
            if let Some(value) = def.get("value") {
                let mut entry = serde_json::Map::new();
                entry.insert("value".to_string(), value.clone());
                out.insert(name.clone(), serde_json::Value::Object(entry));
            }
        }
        serde_json::Value::Object(out).to_string()
    }
}

/// Steam roots to search for a workshop item, most specific first.
fn steam_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    let home = gtk::glib::home_dir();
    for rel in [
        ".local/share/Steam",
        ".steam/steam",
        ".steam/root",
        // Flatpak Steam
        ".var/app/com.valvesoftware.Steam/.local/share/Steam",
    ] {
        roots.push(home.join(rel));
    }
    roots
}

/// Resolve a workshop id to its package directory. Honors `$HOTARU_WPE_WORKSHOP`
/// first, then the standard Steam install locations.
fn resolve_workshop_id(id: &str) -> Result<PathBuf> {
    let mut candidates = Vec::new();

    if let Some(dir) = env::var_os(WORKSHOP_ENV) {
        candidates.push(PathBuf::from(dir).join(id));
    }
    let rel = format!("steamapps/workshop/content/{WPE_WORKSHOP_APP_ID}/{id}");
    for root in steam_roots() {
        candidates.push(root.join(&rel));
    }

    for dir in &candidates {
        if dir.join("project.json").is_file() {
            return Ok(dir.clone());
        }
    }

    bail!(
        "workshop item {} not found (searched {}); set {} to the workshop content directory",
        id,
        candidates
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", "),
        WORKSHOP_ENV
    )
}
