// Copyright © Aptos Foundation

use aptos_metrics_core::{register_int_counter, IntCounter};
use once_cell::sync::Lazy;

/// Number of times the Event Stream has been invoked from a transaction received from PubSub
pub static TRANSACTION_RECEIVED_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "event_stream_transaction_received_count",
        "Number of transactions received by event stream",
    )
    .unwrap()
});

/// Number of times the PubSub subscription stream has been reset
pub static PUBSUB_STREAM_RESET_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "event_stream_pubsub_stream_reset_count",
        "Number of times the PubSub subscription stream has been reset",
    )
    .unwrap()
});

/// Number of times a PubSub message has successfully been ACK'd
pub static PUBSUB_ACK_SUCCESS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "event_stream_pubsub_ack_success_count",
        "Number of times a PubSub message has successfully been ACK'd",
    )
    .unwrap()
});
