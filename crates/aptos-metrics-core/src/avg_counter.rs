// Copyright © Aptos Foundation

use prometheus::{register_histogram, Histogram};

pub struct AverageIntCounter {
    // use histogram, instead of pair of sum/count counters, to guarantee
    // atomicity of observing and fetching (which Histogram handles correctly)
    histogram: Histogram,
}

impl AverageIntCounter {
    pub fn register(name: &str, desc: &str) -> AverageIntCounter {
        Self {
            histogram: register_histogram!(
                name,
                desc,
                // We need to have at least one bucket in histogram, otherwise default buckets are used
                vec![0.5],
            )
            .unwrap(),
        }
    }

    pub fn observe(&self, value: u64) {
        self.histogram.observe(value as f64);
    }
}
