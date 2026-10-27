// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_metrics_core::{exponential_buckets, HistogramVec};
use once_cell::sync::Lazy;
use prometheus::{
    core::{self, Collector},
    histogram_opts,
    proto::MetricFamily,
};
use std::collections::HashSet;
use std::hash::Hash;
use std::time::Instant;

pub const NAMESPACE: &str = "node";

/// Keep the first item for each key in an iterator.
pub(crate) fn first_by_key<T, K>(
    items: impl IntoIterator<Item = T>,
    mut key: impl FnMut(&T) -> K,
) -> impl Iterator<Item = T>
where
    K: Eq + Hash,
{
    let mut seen = HashSet::new();
    items.into_iter().filter(move |item| seen.insert(key(item)))
}

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

#[cfg(test)]
mod tests {
    use super::first_by_key;

    #[test]
    fn first_by_key_keeps_one_item_per_key() {
        let actual: Vec<_> = first_by_key(
            [("/", "8:0"), ("/mnt/a", "8:0"), ("/mnt/b", "8:16")],
            |(_, device)| *device,
        )
        .collect();

        assert_eq!(actual, vec![("/", "8:0"), ("/mnt/b", "8:16")]);
    }
}
