/* gst_utils.rs
 *
 * Copyright 2024 Jeff Shee
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use gst::prelude::{GstObjectExt, PluginFeatureExtManual};

// TODO
static IS_ENABLE_VA: bool = true;
static IS_ENABLE_NVSL: bool = true;

pub fn setup_gst() {
    // Software libav decoders have "primary" rank, set Nvidia higher to use NVDEC hardware acceleration.
    set_plugin_decoder_rank("nvcodec", gst::Rank::PRIMARY + 1);

    // Legacy "vaapidecodebin" have rank "primary + 2", we need to set VA higher then that to be used.
    if IS_ENABLE_VA {
        set_plugin_decoder_rank("va", gst::Rank::PRIMARY + 3);
    }
}

fn set_plugin_decoder_rank(plugin_name: &str, new_rank: gst::Rank) {
    let registry = gst::Registry::get();
    let features = registry.features_by_plugin(plugin_name);
    for feature in features {
        let feature_name = feature.name();
        if !feature_name.ends_with("dec") && !feature_name.ends_with("postproc") {
            continue;
        }

        if plugin_name == "nvcodec" {
            let is_stateless = feature_name.ends_with("sldec");
            if is_stateless != IS_ENABLE_NVSL {
                continue;
            }
        }

        let old_rank = feature.rank();
        if old_rank != new_rank {
            feature.set_rank(new_rank);
            println!(
                "changed rank: {} -> {} for {}",
                old_rank, new_rank, feature_name
            );
        }
    }
}
