// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;
use aptos_metrics_core::{IntGauge, register_int_gauge};

/// Count of the pending messages sent to itself in the channel
pub static PENDING_SELF_MESSAGES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_mpc_pending_self_messages",
        "Count of the pending MPC messages sent to itself in the channel"
    )
        .unwrap()
});
