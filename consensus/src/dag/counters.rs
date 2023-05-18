// Copyright © Aptos Foundation

use aptos_metrics_core::{Histogram, register_histogram, exponential_buckets};
use once_cell::sync::Lazy;


/// Latency
pub static DAG_NODE_TO_BLOCK_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_dag_node_to_block_latency",
        // metric description
        "The time from node creation to node ordering/block creation",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static DAG_NODE_TO_BLOCK_LATENCY_EVEN_ROUND: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_dag_node_to_block_odd_round_latency",
        // metric description
        "The time from node creation to node ordering/block creation",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static DAG_NODE_TO_BLOCK_LATENCY_ODD_ROUND: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_dag_node_to_block_even_round_latency",
        // metric description
        "The time from node creation to node ordering/block creation",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});
