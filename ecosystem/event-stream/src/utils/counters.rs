// Copyright © Aptos Foundation

use aptos_metrics_core::{register_int_counter, IntCounter};
use once_cell::sync::Lazy;

// OVERALL METRICS

/// Number of times the Event Stream has been invoked
pub static EVENT_RECEIVED_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "event_stream_event_received_count",
        "Number of events received by event stream",
    )
    .unwrap()
});

// PUBSUB METRICS

/// Number of times a PubSub message has successfully been ACK'd
pub static PUBSUB_ACK_SUCCESS_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "event_stream_pubsub_ack_success_count",
        "Number of times a PubSub message has successfully been ACK'd",
    )
    .unwrap()
});

// POSTGRES METRICS

/// Number of times the connection pool has timed out when trying to get a connection
pub static UNABLE_TO_GET_CONNECTION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_connection_pool_err",
        "Number of times the connection pool has timed out when trying to get a connection"
    )
    .unwrap()
});

/// Number of times the connection pool got a connection
pub static GOT_CONNECTION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "indexer_connection_pool_ok",
        "Number of times the connection pool got a connection"
    )
    .unwrap()
});

// DEDUPLICATION METRICS

/// Number of times the Event Stream has found a duplicate asset URI
pub static DUPLICATE_ASSET_URI_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "event_stream_duplicate_asset_uri_count",
        "Number of times the Event Stream has found a duplicate asset URI"
    )
    .unwrap()
});

/// Number of times the Event Stream has found a duplicate raw image URI
pub static DUPLICATE_RAW_IMAGE_URI_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "event_stream_duplicate_raw_image_uri_count",
        "Number of times the Event Stream has found a duplicate raw image URI"
    )
    .unwrap()
});

/// Number of times the Event Stream has found a duplicate raw animation URI
pub static DUPLICATE_RAW_ANIMATION_URI_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "event_stream_duplicate_raw_animation_uri_count",
        "Number of times the Event Stream has found a duplicate raw animation URI"
    )
    .unwrap()
});
