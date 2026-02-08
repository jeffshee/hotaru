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

use gst::prelude::*;
use tracing::debug;

pub fn setup_gst(is_enable_va: bool, is_enable_nvsl: bool) {
    // Software libav decoders have "primary" rank, set Nvidia higher to use NVDEC hardware acceleration.
    set_plugin_decoder_rank_nvcodec(gst::Rank::PRIMARY + 1, is_enable_nvsl);

    // Legacy "vaapidecodebin" have rank "primary + 2", we need to set VA higher then that to be used.
    if is_enable_va {
        set_plugin_decoder_rank("va", gst::Rank::PRIMARY + 3);
    }
}

fn set_plugin_decoder_rank_nvcodec(new_rank: gst::Rank, is_enable_nvsl: bool) {
    let registry = gst::Registry::get();
    let features = registry.features_by_plugin("nvcodec");
    for feature in features {
        let feature_name = feature.name();
        if !feature_name.ends_with("dec") && !feature_name.ends_with("postproc") {
            continue;
        }

        let is_stateless = feature_name.ends_with("sldec");
        if is_stateless != is_enable_nvsl {
            continue;
        }

        let old_rank = feature.rank();
        if old_rank != new_rank {
            feature.set_rank(new_rank);
        }
        debug!(
            "changed rank: {} -> {} for {}",
            old_rank, new_rank, feature_name
        );
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

        let old_rank = feature.rank();
        if old_rank != new_rank {
            feature.set_rank(new_rank);
        }
        debug!(
            "changed rank: {} -> {} for {}",
            old_rank, new_rank, feature_name
        );
    }
}
