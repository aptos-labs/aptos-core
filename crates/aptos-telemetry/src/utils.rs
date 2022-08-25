// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_telemetry_service::types::telemetry::TelemetryEvent;
use prometheus::proto::MetricFamily;
use std::collections::BTreeMap;

/// Build information event name
const APTOS_NODE_BUILD_INFORMATION: &str = "APTOS_NODE_BUILD_INFORMATION";
/// Build information keys
pub const BUILD_CHAIN_ID: &str = "build_chain_id";

/// Collects and sends the build information via telemetry
pub(crate) async fn create_build_info_telemetry_event(
    build_info: BTreeMap<String, String>,
) -> TelemetryEvent {
    // Create and return a new telemetry event
    TelemetryEvent {
        name: APTOS_NODE_BUILD_INFORMATION.into(),
        params: build_info,
    }
}

/// Inserts an optional value into the given map iff the value exists
pub(crate) fn insert_optional_value(
    map: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        map.insert(key.to_string(), value);
    }
}

/// Sums all gauge counts in the given set of metric families
pub fn sum_all_gauges(metric_families: &Vec<MetricFamily>) -> f64 {
    let mut gauge_sum = 0.0;
    for metric_family in metric_families {
        for metric in metric_family.get_metric() {
            gauge_sum += metric.get_gauge().get_value();
        }
    }
    gauge_sum
}

/// Sums all histogram sample counts in the given set of metric families
pub fn sum_all_histogram_counts(metric_families: &Vec<MetricFamily>) -> f64 {
    let mut count_sum = 0.0;
    for metric_family in metric_families {
        for metric in metric_family.get_metric() {
            count_sum += metric.get_histogram().get_sample_count() as f64
        }
    }
    count_sum
}

/// Sums all histogram sample sums in the given set of metric families
pub fn sum_all_histogram_sums(metric_families: &Vec<MetricFamily>) -> f64 {
    let mut count_sum = 0.0;
    for metric_family in metric_families {
        for metric in metric_family.get_metric() {
            count_sum += metric.get_histogram().get_sample_sum() as f64
        }
    }
    count_sum
}
