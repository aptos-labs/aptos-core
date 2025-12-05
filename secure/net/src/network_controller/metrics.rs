// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{exponential_buckets, register_histogram_vec, HistogramVec};
use once_cell::sync::Lazy;

pub static NETWORK_HANDLER_TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "network_handler_timer",
        // metric description
        "The time spent in processing: \
         1. outbound_msgs: sending messages to remote nodes; \
         2. inbound_msgs: routing inbound messages to respective handlers;",
        // metric labels (dimensions)
        &["node_addr", "name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});
