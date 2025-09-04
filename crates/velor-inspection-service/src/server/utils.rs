// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_logger::{error, warn};
use velor_metrics_core::{register_int_counter_vec, IntCounterVec};
use once_cell::sync::Lazy;
use prometheus::{
    proto::{MetricFamily, MetricType},
    Encoder,
};
use std::collections::HashMap;

// Useful string constants
pub const CONTENT_TYPE_JSON: &str = "application/json";
pub const CONTENT_TYPE_TEXT: &str = "text/plain";

/// Counter for the number of metrics in various states
pub static NUM_METRICS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!("velor_metrics", "Number of metrics in certain states", &[
        "type"
    ])
    .unwrap()
});

/// A simple utility function that returns all metrics as a HashMap
pub fn get_all_metrics() -> HashMap<String, String> {
    let metric_families = get_metric_families();
    get_metrics_map(metric_families)
}

/// A simple utility function that encodes the metrics using the given encoder
pub fn get_encoded_metrics(encoder: impl Encoder) -> Vec<u8> {
    // Gather and encode the metrics
    let metric_families = get_metric_families();
    let mut encoded_buffer = vec![];
    if let Err(error) = encoder.encode(&metric_families, &mut encoded_buffer) {
        error!("Failed to encode metrics! Error: {}", error);
        return vec![];
    }

    // Update the total metric bytes counter
    NUM_METRICS
        .with_label_values(&["total_bytes"])
        .inc_by(encoded_buffer.len() as u64);

    encoded_buffer
}

/// A simple utility function that returns all metric families
fn get_metric_families() -> Vec<MetricFamily> {
    let metric_families = velor_metrics_core::gather();
    let mut total: u64 = 0;
    let mut families_over_2000: u64 = 0;

    // Take metrics of metric gathering so we know possible overhead of this process
    for metric_family in &metric_families {
        let family_count = metric_family.get_metric().len();
        if family_count > 2000 {
            families_over_2000 = families_over_2000.saturating_add(1);
            let name = metric_family.get_name();
            warn!(
                count = family_count,
                metric_family = name,
                "Metric Family '{}' over 2000 dimensions '{}'",
                name,
                family_count
            );
        }
        total = total.saturating_add(family_count as u64);
    }

    // These metrics will be reported on the next pull, rather than create a new family
    NUM_METRICS.with_label_values(&["total"]).inc_by(total);
    NUM_METRICS
        .with_label_values(&["families_over_2000"])
        .inc_by(families_over_2000);

    metric_families
}

/// A simple utility function that parses and collects all metrics
/// associated with the given families.
fn get_metrics_map(metric_families: Vec<MetricFamily>) -> HashMap<String, String> {
    // TODO: use an existing metric encoder (same as used by prometheus/metric-server)
    let mut all_metrics = HashMap::new();

    // Process each metric family
    for metric_family in metric_families {
        let values: Vec<_> = match metric_family.get_field_type() {
            MetricType::COUNTER => metric_family
                .get_metric()
                .iter()
                .map(|m| m.get_counter().get_value().to_string())
                .collect(),
            MetricType::GAUGE => metric_family
                .get_metric()
                .iter()
                .map(|m| m.get_gauge().get_value().to_string())
                .collect(),
            MetricType::SUMMARY => {
                error!("Unsupported Metric 'SUMMARY'");
                vec![]
            },
            MetricType::UNTYPED => {
                error!("Unsupported Metric 'UNTYPED'");
                vec![]
            },
            MetricType::HISTOGRAM => metric_family
                .get_metric()
                .iter()
                .map(|m| m.get_histogram().get_sample_count().to_string())
                .collect(),
        };
        let metric_names = metric_family.get_metric().iter().map(|m| {
            let label_strings: Vec<String> = m
                .get_label()
                .iter()
                .map(|l| format!("{}={}", l.get_name(), l.get_value()))
                .collect();
            let labels_string = format!("{{{}}}", label_strings.join(","));
            format!("{}{}", metric_family.get_name(), labels_string)
        });

        for (name, value) in metric_names.zip(values.into_iter()) {
            all_metrics.insert(name, value);
        }
    }

    all_metrics
}
