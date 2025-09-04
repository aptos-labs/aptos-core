// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{
    exponential_buckets, register_histogram_vec, register_int_counter_vec, HistogramVec,
    IntCounterVec,
};
use once_cell::sync::Lazy;

pub static REMOTE_EXECUTOR_TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "remote_executor_timer",
        // metric description
        "The time spent in remote shard on: \
         1. cmd_rx: after receiving the command from the coordinator; \
         2. cmd_rx_bcs_deser: deserializing the received command; \
         3. init_prefetch: initializing the prefetching of remote state values \
         4. kv_responses: processing the remote key value responses; \
         5. kv_resp_deser: deserializing the remote key value responses; \
         6. prefetch_wait: waiting (approx) for the remote state values to be prefetched; \
         7. non_prefetch_wait: waiting for the remote state values that were not prefetched; \
         8. kv_req_deser: deserializing the remote key value requests; \
         9. kv_requests: processing the remote key value requests; \
         10. kv_resp_ser: serializing the remote key value responses;",
        // metric labels (dimensions)
        &["shard_id", "name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static REMOTE_EXECUTOR_REMOTE_KV_COUNT: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        // metric name
        "remote_executor_remote_kv_count",
        // metric description
        "KV counts on a shard for: \
         1. kv_responses: the number of remote key value responses received on a shard; \
         2. non_prefetch_kv: the number of remote key value responses received on a shard that were not prefetched; \
         3. prefetch_kv: the number of remote key value responses received on a shard that were prefetched; ",
        // metric labels (dimensions)
        &["shard_id", "name"],
    )
    .unwrap()
});
