// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::Mutex;
use aptos_metrics_core::{register_int_gauge_vec, IntGaugeVec};
use once_cell::sync::Lazy;
use sysinfo::{DiskExt, System, SystemExt};

/// Global system singleton (to avoid recreations)
pub static GLOBAL_SYSTEM: Lazy<Mutex<System>> = Lazy::new(|| Mutex::new(System::new_all()));

/// Total available system disk space (bytes)
static AVAILABLE_DISK_SPACE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "system_available_disk_space",
        "Available system disk space (bytes)",
        &[]
    )
    .unwrap()
});

/// Total available system RAM (KB)
static AVAILABLE_MEMORY_GAUGE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "system_available_memory",
        "Available system memory (KB)",
        &[]
    )
    .unwrap()
});

/// Total number of CPU cores
static PHYSICAL_CORE_COUNT_GAUGE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!("system_physical_core_count", "Physical CPU core count", &[]).unwrap()
});

/// Total system disk space (bytes)
static TOTAL_DISK_SPACE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "system_total_disk_space",
        "Total system disk space (bytes)",
        &[]
    )
    .unwrap()
});

/// Total system RAM (KB)
static TOTAL_MEMORY_GAUGE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!("system_total_memory", "Total system memory (KB)", &[]).unwrap()
});

/// Used system RAM (KB)
static USED_MEMORY_GAUGE: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!("system_used_memory", "Used system memory (KB)", &[]).unwrap()
});

/// Updates all system metrics that are being tracked (e.g., by telemetry).
/// These metrics must be gauges or counters.
pub fn update_system_metrics() {
    // Note: this might be expensive, so it shouldn't be done often.
    GLOBAL_SYSTEM.lock().refresh_all();

    // Update the metrics
    update_cpu_metrics(&GLOBAL_SYSTEM);
    update_memory_metrics(&GLOBAL_SYSTEM);
    update_disk_metrics(&GLOBAL_SYSTEM);
}

/// Updates all CPU related metrics
fn update_cpu_metrics(system: &Lazy<Mutex<System>>) {
    if let Some(physical_core_count) = system.lock().physical_core_count() {
        PHYSICAL_CORE_COUNT_GAUGE
            .with_label_values(&[])
            .set(physical_core_count as i64);
    }
}

/// Updates all memory related metrics
fn update_memory_metrics(system: &Lazy<Mutex<System>>) {
    AVAILABLE_MEMORY_GAUGE
        .with_label_values(&[])
        .set(system.lock().available_memory() as i64);
    TOTAL_MEMORY_GAUGE
        .with_label_values(&[])
        .set(system.lock().total_memory() as i64);
    USED_MEMORY_GAUGE
        .with_label_values(&[])
        .set(system.lock().used_memory() as i64);
}

/// Updates all disk (storage) related metrics
fn update_disk_metrics(system: &Lazy<Mutex<System>>) {
    let mut total_disk_space = 0;
    let mut available_disk_space = 0;
    for disk in system.lock().disks() {
        total_disk_space += disk.total_space();
        available_disk_space += disk.available_space();
    }

    AVAILABLE_DISK_SPACE
        .with_label_values(&[])
        .set(available_disk_space as i64);
    TOTAL_DISK_SPACE
        .with_label_values(&[])
        .set(total_disk_space as i64);
}
