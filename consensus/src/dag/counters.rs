// Copyright © Aptos Foundation

use aptos_metrics_core::{Histogram, register_histogram, exponential_buckets, register_counter, register_int_counter, IntCounter};
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

pub static DAG_NODE_TO_BLOCK_LATENCY_EVEN_ROUND_MIN: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_dag_node_to_block_odd_round_latency_min",
        // metric description
        "The time from node creation to node ordering/block creation",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static DAG_NODE_TO_BLOCK_LATENCY_ODD_ROUND_MIN: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_dag_node_to_block_even_round_latency_min",
        // metric description
        "The time from node creation to node ordering/block creation",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

const DAG_NODE_ROUND_DIFF_BUCKETS: [f64; 20] = [
    0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0
];

pub static DAG_NODE_ROUND_DIFF: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        "aptos_dag_node_round_diff",
        "The diff between rounds from anchor round that committed this node",
        DAG_NODE_ROUND_DIFF_BUCKETS.to_vec()
    )
    .unwrap()
});

pub static DAG_NODE_TO_BLOCK_SAME_AUTHOR_LATENCY: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_dag_node_to_block_same_author_latency",
        // metric description
        "The time from node creation to node ordering/block creation same author nodes",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static DAG_NODE_TO_BLOCK_SAME_AUTHOR_LATENCY_EVEN_ROUND: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_dag_node_to_block_same_author_even_round_latency",
        // metric description
        "The time from node creation to node ordering/block creation same author nodes",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static DAG_NODE_TO_BLOCK_SAME_AUTHOR_LATENCY_ODD_ROUND: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_dag_node_to_block_same_author_odd_round_latency",
        // metric description
        "The time from node creation to node ordering/block creation same author nodes",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static DAG_NODE_TO_BLOCK_SAME_AUTHOR_LATENCY_EVEN_ROUND_MIN: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_dag_node_to_block_same_author_even_round_latency_min",
        // metric description
        "The time from node creation to node ordering/block creation same author nodes",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});

pub static DAG_NODE_TO_BLOCK_SAME_AUTHOR_LATENCY_ODD_ROUND_MIN: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(
        // metric name
        "aptos_dag_node_to_block_same_author_odd_round_latency_min",
        // metric description
        "The time from node creation to node ordering/block creation same author nodes",
        exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 30).unwrap(),
    )
    .unwrap()
});
