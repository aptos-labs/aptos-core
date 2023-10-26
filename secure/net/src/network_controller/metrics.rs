// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{exponential_buckets, register_histogram_vec, HistogramVec};
use once_cell::sync::Lazy;

pub static NETWORK_HANDLER_TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "network_handler_timer",
        // metric description
        "The time spent in processing: \
         1. outbound_msgs: sending messages to remote nodes; \
         2. inbound_msgs: routing inbound messages to respective handlers;\
         3. outbound_msgs_full_loop: time spent in receiving and sending outgoing messages",
        // metric labels (dimensions)
        &["node_addr", "name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    )
    .unwrap()
});

pub static REMOTE_EXECUTOR_RND_TRP_JRNY_TIMER: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        // metric name
        "remote_executor_rnd_trp_jrny_timer",
        // metric description
        "Time spent since the key was put in HT and remote call was initiated to fetch the value: \
        1. 1_kv_req_grpc_shard_send; \
        2. 2_kv_req_coord_grpc_recv; \
        3. 3_kv_req_coord_handler_st; \
        4. 4_kv_req_coord_handler_end; \
        5. 5_kv_resp_coord_grpc_send;\
        6. 6_kv_resp_shard_grpc_recv; \
        7. 7_kv_resp_shard_handler_st; \
        8. 8_kv_resp_shard_handler_end; ",
        // metric labels (dimensions)
        &["name"],
        exponential_buckets(/*start=*/ 1e-3, /*factor=*/ 2.0, /*count=*/ 20).unwrap(),
    ).unwrap()
});
