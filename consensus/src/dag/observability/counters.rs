// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_histogram_vec, register_int_gauge, HistogramVec, IntGauge};
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
