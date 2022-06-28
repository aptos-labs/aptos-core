// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{build_information::collect_build_info, system_information::collect_system_info};
use prometheus::proto::MetricFamily;
use std::collections::BTreeMap;

/// Used to expose system and build information
pub fn get_system_and_build_information(chain_id: Option<String>) -> BTreeMap<String, String> {
    let mut information: BTreeMap<String, String> = BTreeMap::new();
    collect_build_info(chain_id, &mut information);
    collect_system_info(&mut information);
    information
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
