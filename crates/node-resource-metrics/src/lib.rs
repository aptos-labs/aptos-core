// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::Mutex;
use cfg_if::cfg_if;
use collectors::{
    CollectorLatencyCollector, CpuMetricsCollector, DiskMetricsCollector, LoadAvgCollector,
    MemoryMetricsCollector, NetworkMetricsCollector, ProcessMetricsCollector,
};
use once_cell::sync::Lazy;

mod collectors;

static IS_REGISTERED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

/// Registers the node metrics collector with the default registry.
pub fn register_node_metrics_collector() {
    let mut registered = IS_REGISTERED.lock();
    if *registered {
        return;
    } else {
        *registered = true;
    }

    prometheus::register(Box::new(CpuMetricsCollector::default())).unwrap();
    prometheus::register(Box::new(MemoryMetricsCollector::default())).unwrap();
    prometheus::register(Box::new(DiskMetricsCollector::default())).unwrap();
    prometheus::register(Box::new(NetworkMetricsCollector::default())).unwrap();
    prometheus::register(Box::new(LoadAvgCollector::default())).unwrap();
    prometheus::register(Box::new(ProcessMetricsCollector::default())).unwrap();
    cfg_if! {
        if #[cfg(all(target_os="linux"))] {
            prometheus::register(Box::new(collectors::LinuxCpuMetricsCollector::default())).unwrap();
            prometheus::register(Box::new(collectors::LinuxDiskMetricsCollector::default())).unwrap();
        }
    }
    prometheus::register(Box::new(CollectorLatencyCollector::default())).unwrap();
}
