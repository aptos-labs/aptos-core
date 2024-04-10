// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    register_histogram, register_histogram_vec, register_int_counter, register_int_counter_vec,
    register_int_gauge, Histogram, HistogramVec, IntCounter, IntCounterVec, IntGauge,
};
use once_cell::sync::Lazy;

/// Traces node latency movement throughout the DAG
pub static NODE_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_consensus_dag_node_tracing",
        "Histogram for different stages of a node",
        &["stage"]
    )
    .unwrap()
});

/// Traces round latency movement throughout the DAG
pub static ROUND_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "aptos_consensus_dag_round_tracing",
        "Histogram for different stages of a round",
        &["stage"]
    )
    .unwrap()
});

/// This counter is set to the last round reported by the local round_state.
pub static CURRENT_ROUND: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_consensus_dag_current_round",
        "This counter is set to the last round reported by the dag driver."
    )
    .unwrap()
});

const NUM_CONSENSUS_TRANSACTIONS_BUCKETS: [f64; 24] = [
    5.0, 10.0, 20.0, 40.0, 75.0, 100.0, 200.0, 400.0, 800.0, 1200.0, 1800.0, 2500.0, 3300.0,
    4000.0, 5000.0, 6500.0, 8000.0, 10000.0, 12500.0, 15000.0, 18000.0, 21000.0, 25000.0, 30000.0,
];

pub static NUM_TXNS_PER_NODE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_dag_num_txns_per_node",
        "Histogram counting the number of transactions per node",
        NUM_CONSENSUS_TRANSACTIONS_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static NODE_PAYLOAD_SIZE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_dag_node_payload_size",
        "Histogram counting the size of the node payload",
    )
    .unwrap()
});

pub static NUM_NODES_PER_BLOCK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_dag_num_nodes_per_block",
        "Histogram counting the number of nodes per block",
    )
    .unwrap()
});

pub static NUM_ROUNDS_PER_BLOCK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_dag_num_rounds_per_block",
        "Histogram counting the number of rounds per block",
    )
    .unwrap()
});

pub static RB_HANDLE_ACKS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_dag_rb_handle_acks",
        "Count of number of RB Handler Acks returned."
    )
    .unwrap()
});

pub static ANCHOR_ORDER_TYPE: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_dag_anchor_order_type",
        "Number of anchors ordered",
        &["mode"]
    )
    .unwrap()
});

pub static FETCH_ENQUEUE_FAILURES: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_dag_fetch_req_enq_failure",
        "Fetch request failed",
        &["type"]
    )
    .unwrap()
});

pub static DAG_RPC_CHANNEL: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_consensus_dag_rpc_channel",
        "Counters(queued,dequeued,dropped) related to dag channel",
        &["state"]
    )
    .unwrap()
});

pub static INCOMING_MSG_PROCESSING: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_consensus_dag_incoming_msg_process",
        "dag incoming message processing"
    )
    .unwrap()
});

pub static TIMEOUT_WAIT_VOTING_POWER_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_consensus_dag_round_timeout_count",
        "round timeout count"
    )
    .unwrap()
});
