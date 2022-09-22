// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::Instant;

use aptos_metrics_core::{exponential_buckets, HistogramVec};
use once_cell::sync::Lazy;
use prometheus::{
    core::{self, Collector},
    histogram_opts,
    proto::MetricFamily,
};

pub const NAMESPACE: &str = "node";

pub static LATENCY: Lazy<HistogramVec> = Lazy::new(|| {
    HistogramVec::new(
        histogram_opts!(
            "node_resource_metrics_collect_latency_micros",
            "Latency to collect each node resource metric category.",
            exponential_buckets(/*start=*/ 10.0, /*factor=*/ 2.0, /*count=*/ 12,).unwrap(),
        ),
        &["collector"],
    )
    .unwrap()
});

/// Collector for collector latency stats
#[derive(Default)]
pub(crate) struct CollectorLatencyCollector;

impl Collector for CollectorLatencyCollector {
    fn desc(&self) -> Vec<&core::Desc> {
        LATENCY.desc()
    }

    fn collect(&self) -> Vec<MetricFamily> {
        LATENCY.collect()
    }
}

/// Provides ability to measure latency and observe it in an histogram
pub(crate) struct MeasureLatency {
    start: Instant,
    name: String,
}

impl MeasureLatency {
    pub fn new(name: String) -> Self {
        Self {
            start: Instant::now(),
            name,
        }
    }
}

impl Drop for MeasureLatency {
    fn drop(&mut self) {
        let elapsed = self.start.elapsed().as_micros();
        LATENCY
            .with_label_values(&[&self.name])
            .observe(elapsed as f64);
    }
}
