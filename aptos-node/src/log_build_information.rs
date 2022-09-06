// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_build_info::build_information;
use aptos_logger::info;

macro_rules! log_feature_info {
    ($($feature:literal),*) => {
        $(
        if cfg!(feature = $feature) {
            info!("Running with {} feature enabled", $feature);
        } else {
            info!("Running with {} feature disabled", $feature);
        }
        )*
    }
}

// Naturally this should only be called after a logger has been set up.
pub fn log_build_information() {
    info!("Build information:");
    let build_info = build_information!();
    for (key, value) in build_info {
        info!("{}: {}", key, value);
    }
    info!("Feature information:");
    log_feature_info!("failpoints", "assert-private-keys-not-cloneable");
}
