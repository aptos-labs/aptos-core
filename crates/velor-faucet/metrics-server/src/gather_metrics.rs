// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_logger::prelude::*;
use velor_metrics_core::{register_int_counter_vec, IntCounterVec};
use once_cell::sync::Lazy;

pub static NUM_METRICS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!("velor_metrics", "Number of metrics in certain states", &[
        "type"
    ])
    .unwrap()
});

pub fn gather_metrics() -> Vec<prometheus::proto::MetricFamily> {
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
