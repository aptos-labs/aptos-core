// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::CONTENT_TYPE_JSON;
use velor_logger::error;
use prometheus::{
    proto::{LabelPair, Metric, MetricFamily, MetricType},
    Encoder, Result,
};
use std::{collections::HashMap, io::Write};

// TODO: figure out if we really need all metric endpoints...

/// An implementation of an [`Encoder`](::Encoder) that converts a `MetricFamily` proto message
/// into `fbagent` json.
///
/// This implementation converts metric{dimensions,...} -> value to a flat string with a value.
/// e.g., `"requests{method="GET", service="accounts"} -> 8` into `requests.GET.account -> 8`.
/// For now, it ignores timestamps (if set on the metric).
#[derive(Debug, Default)]
pub struct JsonEncoder;

impl Encoder for JsonEncoder {
    fn encode<W: Write>(&self, metric_families: &[MetricFamily], writer: &mut W) -> Result<()> {
        let mut encoded_metrics: HashMap<String, f64> = HashMap::new();

        // Go through each metric family and encode it
        for metric_family in metric_families {
            let name = metric_family.get_name();
            let metric_type = metric_family.get_field_type();
            for metric in metric_family.get_metric() {
                match metric_type {
                    MetricType::COUNTER => {
                        encoded_metrics.insert(
                            flatten_metric_with_labels(name, metric),
                            metric.get_counter().get_value(),
                        );
                    },
                    MetricType::GAUGE => {
                        encoded_metrics.insert(
                            flatten_metric_with_labels(name, metric),
                            metric.get_gauge().get_value(),
                        );
                    },
                    MetricType::HISTOGRAM => {
                        // write the sum and counts
                        let h = metric.get_histogram();
                        encoded_metrics.insert(
                            flatten_metric_with_labels(&format!("{}_count", name), metric),
                            h.get_sample_count() as f64,
                        );
                        encoded_metrics.insert(
                            flatten_metric_with_labels(&format!("{}_sum", name), metric),
                            h.get_sample_sum(),
                        );
                    },
                    _ => {
                        // Do nothing (not supported)
                    },
                }
            }
        }

        // Write the encoded metrics to the writer
        match serde_json::to_string(&encoded_metrics) {
            Ok(json_encoded_metrics) => {
                writer.write_all(json_encoded_metrics.as_bytes())?;
            },
            Err(error) => {
                error!("Failed to JSON encode the metrics! Error: {}", error);
            },
        };

        Ok(())
    }

    fn format_type(&self) -> &str {
        CONTENT_TYPE_JSON
    }
}

/**
This method takes Prometheus metrics with dimensions (represented as label:value tags)
and converts it into a dot-separated string.

Example:
Prometheus metric: error_count{method: "get_account", error="connection_error"}
Result: error_count.get_account.connection_error

If the set of labels is empty, only the name is returned
Example:
Prometheus metric: errors
Result: errors

This is useful when exporting metric data to flat time series.
*/
fn flatten_metric_with_labels(name: &str, metric: &Metric) -> String {
    // If the metric has no labels, return the name
    let name_string = String::from(name);
    if metric.get_label().is_empty() {
        return name_string;
    }

    // Join the values of the labels with "."
    let values: Vec<&str> = metric
        .get_label()
        .iter()
        .map(LabelPair::get_value)
        .filter(|&x| !x.is_empty())
        .collect();
    let values = values.join(".");

    // If the values are empty, return the name
    if values.is_empty() {
        return name_string;
    }

    // Otherwise, return the name with the values
    format!("{}.{}", name_string, values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::{
        core::{Collector, Metric},
        IntCounter, IntCounterVec, Opts,
    };
    use serde_json::Value;

    #[test]
    fn test_flatten_labels() {
        // Generate a counter for testing
        let counter_name_1 = "counter_1";
        let counter_1 = IntCounter::new(counter_name_1, "Test counter 1").unwrap();

        // Flatten the metric and check the result
        let flattened_metric = flatten_metric_with_labels(counter_name_1, &counter_1.metric());
        assert_eq!(flattened_metric, counter_name_1.to_string());

        // Generate another counter for testing
        let counter_name_2 = "counter_2";
        let counter_2 =
            IntCounterVec::new(Opts::new(counter_name_2, "Test counter 2"), &["label_me"]).unwrap();

        // Flatten the metric (without a label) and check the result
        let flattened_metric = flatten_metric_with_labels(
            counter_name_2,
            &counter_2.with_label_values(&[""]).metric(),
        );
        assert_eq!(flattened_metric, counter_name_2.to_string());

        // Flatten the metric (with a label) and check the result
        let flattened_metric = flatten_metric_with_labels(
            counter_name_2,
            &counter_2.with_label_values(&["hello"]).metric(),
        );
        assert_eq!(flattened_metric, "counter_2.hello".to_string());

        // Generate another counter for testing
        let another_counter_2 =
            IntCounterVec::new(Opts::new(counter_name_2, "Example counter for testing"), &[
                "label_me",
                "label_me_too",
            ])
            .unwrap();

        // Flatten a mismatched metric (without a label) and check the result
        let counter_name_3 = "counter_3";
        let flattened_metric = flatten_metric_with_labels(
            counter_name_3,
            &another_counter_2.with_label_values(&["", ""]).metric(),
        );
        assert_eq!(flattened_metric, counter_name_3.to_string());

        // Flatten a mismatched metric (with a label) and check the result
        let flattened_metric = flatten_metric_with_labels(
            counter_name_3,
            &another_counter_2
                .with_label_values(&["hello", "world"])
                .metric(),
        );
        assert_eq!(flattened_metric, "counter_3.hello.world");
    }

    #[test]
    fn test_encoder() {
        // Generate a counter for testing
        let counter = IntCounterVec::new(Opts::new("testing_count", "Test Counter"), &[
            "method", "result",
        ])
        .unwrap();

        // Add test data to the counter
        counter.with_label_values(&["get", "302"]).inc();
        counter.with_label_values(&["get", "302"]).inc();
        counter.with_label_values(&["get", "404"]).inc();
        counter.with_label_values(&["put", ""]).inc();

        // Get the counter data and JSON encode it
        let metric_family = counter.collect();
        let mut data_writer = Vec::<u8>::new();
        let res = JsonEncoder.encode(&metric_family, &mut data_writer);
        assert!(res.is_ok());

        // Decode the JSON and check the result
        let decoded_value: Value = serde_json::from_slice(&data_writer).unwrap();
        let expected_json: &str = r#"
        {
            "testing_count.get.302": 2.0,
            "testing_count.get.404": 1.0,
            "testing_count.put": 1.0
        }"#;
        let expected_value: Value = serde_json::from_str(expected_json).unwrap();
        assert_eq!(decoded_value, expected_value);
    }
}
