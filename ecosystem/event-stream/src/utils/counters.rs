// Copyright © Aptos Foundation

use aptos_metrics_core::{register_int_counter, IntCounter};
use once_cell::sync::Lazy;

/// Number of times the Event Stream has been invoked
pub static EVENT_RECEIVED_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "event_stream_event_received_count",
        "Number of events received by event stream",
    )
    .unwrap()
});
