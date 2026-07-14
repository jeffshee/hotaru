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

//! Build-time configuration. Meson injects the authoritative values via
//! `HOTARU_VERSION` / `HOTARU_PKGDATADIR` when it drives cargo (see
//! src/meson.build); plain `cargo build` falls back to the cargo package
//! version and the standard system data directory.

pub const VERSION: &str = match option_env!("HOTARU_VERSION") {
    Some(version) => version,
    None => env!("CARGO_PKG_VERSION"),
};

/// Installed data directory (gresource bundle etc.).
#[allow(dead_code)] // not read yet; kept for gresource loading
pub const PKGDATADIR: &str = match option_env!("HOTARU_PKGDATADIR") {
    Some(dir) => dir,
    None => "/usr/share/hotaru",
};
