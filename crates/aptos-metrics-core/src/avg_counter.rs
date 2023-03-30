// Copyright © Aptos Foundation

use prometheus::{register_histogram, Histogram};

// use histogram, instead of pair of sum/count counters, to guarantee
// atomicity of observing and fetching (which Histogram handles correctly)
pub fn register_avg_counter(name: &str, desc: &str) -> Histogram {
    register_histogram!(
        name,
        desc,
        // We need to have at least one bucket in histogram, otherwise default buckets are used
        vec![0.5],
    )
    .unwrap()
}
