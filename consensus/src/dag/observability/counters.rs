// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::unwrap_used)]

use velor_metrics_core::{
    register_histogram, register_histogram_vec, register_int_gauge, Histogram, HistogramVec,
    IntGauge,
};
use once_cell::sync::Lazy;

/// Traces node latency movement throughout the DAG
pub static NODE_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_consensus_dag_node_tracing",
        "Histogram for different stages of a node",
        &["stage"]
    )
    .unwrap()
});

/// Traces round latency movement throughout the DAG
pub static ROUND_TRACING: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_consensus_dag_round_tracing",
        "Histogram for different stages of a round",
        &["stage"]
    )
    .unwrap()
});

/// This counter is set to the last round reported by the local round_state.
pub static CURRENT_ROUND: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_consensus_dag_current_round",
        "This counter is set to the last round reported by the dag driver."
    )
    .unwrap()
});

pub static NUM_TXNS_PER_NODE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "velor_consensus_dag_num_txns_per_node",
        "Histogram counting the number of transactions per node",
    )
    .unwrap()
});

pub static NODE_PAYLOAD_SIZE: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "velor_consensus_dag_node_payload_size",
        "Histogram counting the size of the node payload",
    )
    .unwrap()
});

pub static NUM_NODES_PER_BLOCK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "velor_consensus_dag_num_nodes_per_block",
        "Histogram counting the number of nodes per block",
    )
    .unwrap()
});

pub static NUM_ROUNDS_PER_BLOCK: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "velor_consensus_dag_num_rounds_per_block",
        "Histogram counting the number of rounds per block",
    )
    .unwrap()
});
