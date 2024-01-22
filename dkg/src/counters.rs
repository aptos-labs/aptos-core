// Copyright © Aptos Foundation

use aptos_metrics_core::{register_int_gauge, IntGauge};
use once_cell::sync::Lazy;

/// Count of the pending messages sent to itself in the channel
pub static PENDING_SELF_MESSAGES: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_dkg_pending_self_messages",
        "Count of the pending messages sent to itself in the channel"
    )
    .unwrap()
});
