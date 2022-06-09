// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_gauge_vec, IntGaugeVec};
use once_cell::sync::Lazy;
use sysinfo::{System, SystemExt};

static TOTAL_MEMORY_GAUGE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!("system_total_memory", "Total system memory", &[]).unwrap()
});

static USED_MEMORY_GAUGE: Lazy<IntGaugeVec> =
    Lazy::new(|| register_int_gauge_vec!("system_used_memory", "Used system memory", &[]).unwrap());

static PHYSICAL_CORE_COUNT_GAUGE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!("system_physical_core_count", "Physical CPU cores", &[]).unwrap()
});

pub fn refresh_system_metrics() {
    let mut sys = System::new();
    sys.refresh_system();

    TOTAL_MEMORY_GAUGE
        .with_label_values(&[])
        .set(sys.total_memory() as i64);
    USED_MEMORY_GAUGE
        .with_label_values(&[])
        .set(sys.used_memory() as i64);

    if let Some(physical_core_count) = sys.physical_core_count() {
        PHYSICAL_CORE_COUNT_GAUGE
            .with_label_values(&[])
            .set(physical_core_count as i64);
    }
}
