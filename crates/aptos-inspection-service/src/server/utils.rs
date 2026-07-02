// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_logger::{error, warn};
use aptos_metrics_core::{register_int_counter_vec, IntCounterVec};
use once_cell::sync::Lazy;
use prometheus::{
    proto::{Metric, MetricFamily, MetricType},
    Encoder,
};
use std::collections::HashMap;

// Useful string constants
pub const CONTENT_TYPE_JSON: &str = "application/json";
pub const CONTENT_TYPE_TEXT: &str = "text/plain";

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum FlattenedMetricValue {
    Single(f64),
    Histogram { count: u64, sum: f64 },
}

/// Counter for the number of metrics in various states
pub static NUM_METRICS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "aptos_metrics",
        "Number of metrics in certain states",
        &["type"]
    )
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
    let metric_families = aptos_metrics_core::gather();
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

pub(crate) fn format_metric_with_labels(name: &str, metric: &Metric) -> String {
    let label_strings: Vec<String> = metric
        .get_label()
        .iter()
        .map(|label| format!("{}={}", label.get_name(), label.get_value()))
        .collect();
    let labels_string = format!("{{{}}}", label_strings.join(","));
    format!("{}{}", name, labels_string)
}

pub(crate) fn flatten_metric_with_labels(name: &str, metric: &Metric) -> String {
    let name_string = String::from(name);
    if metric.get_label().is_empty() {
        return name_string;
    }

    let values: Vec<&str> = metric
        .get_label()
        .iter()
        .map(|label| label.get_value())
        .filter(|value| !value.is_empty())
        .collect();
    let values = values.join(".");

    if values.is_empty() {
        return name_string;
    }

    format!("{}.{}", name_string, values)
}

pub(crate) fn for_each_flattened_metric<F>(metric_families: &[MetricFamily], mut visitor: F)
where
    F: FnMut(&str, &Metric, FlattenedMetricValue),
{
    for metric_family in metric_families {
        let name = metric_family.get_name();
        match metric_family.get_field_type() {
            MetricType::COUNTER => {
                for metric in metric_family.get_metric() {
                    visitor(
                        name,
                        metric,
                        FlattenedMetricValue::Single(metric.get_counter().get_value()),
                    );
                }
            },
            MetricType::GAUGE => {
                for metric in metric_family.get_metric() {
                    visitor(
                        name,
                        metric,
                        FlattenedMetricValue::Single(metric.get_gauge().get_value()),
                    );
                }
            },
            MetricType::HISTOGRAM => {
                for metric in metric_family.get_metric() {
                    let histogram = metric.get_histogram();
                    visitor(
                        name,
                        metric,
                        FlattenedMetricValue::Histogram {
                            count: histogram.get_sample_count(),
                            sum: histogram.get_sample_sum(),
                        },
                    );
                }
            },
            MetricType::SUMMARY => error!("Unsupported Metric 'SUMMARY'"),
            MetricType::UNTYPED => error!("Unsupported Metric 'UNTYPED'"),
        }
    }
}

/// A simple utility function that parses and collects all metrics
/// associated with the given families.
fn get_metrics_map(metric_families: Vec<MetricFamily>) -> HashMap<String, String> {
    let mut all_metrics = HashMap::new();

    for_each_flattened_metric(&metric_families, |name, metric, value| {
        let key = format_metric_with_labels(name, metric);
        let value = match value {
            FlattenedMetricValue::Single(value) => value.to_string(),
            FlattenedMetricValue::Histogram { count, .. } => count.to_string(),
        };
        all_metrics.insert(key, value);
    });

    all_metrics
}
