// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_counter, Counter};
use once_cell::sync::Lazy;

pub static UPDATE_CNT: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "aptos_active_state_set_update_cnt",
        "Number of updates in the active state set.",
    )
    .unwrap()
});
