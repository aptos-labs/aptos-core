// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use cfg_if::cfg_if;
use collectors::{
    CpuMetricsCollector, DiskMetricsCollector, LinuxCpuMetricsCollector, LinuxDiskMetricsCollector,
    LinuxProcessMetricsCollector, LoadAvgCollector, MemoryMetricsCollector,
    NetworkMetricsCollector, ProcessMetricsCollector,
};

mod collectors;

/// Registers the node metrics collector with the default registry.
pub fn register_node_metrics_collector() {
    prometheus::register(Box::new(CpuMetricsCollector::default())).unwrap();
    prometheus::register(Box::new(MemoryMetricsCollector::default())).unwrap();
    prometheus::register(Box::new(DiskMetricsCollector::default())).unwrap();
    prometheus::register(Box::new(NetworkMetricsCollector::default())).unwrap();
    prometheus::register(Box::new(LoadAvgCollector::default())).unwrap();
    prometheus::register(Box::new(ProcessMetricsCollector::default())).unwrap();
    cfg_if! {
        if #[cfg(all(target_os="linux"))] {
            prometheus::register(Box::new(LinuxCpuMetricsCollector::default())).unwrap();
            prometheus::register(Box::new(LinuxDiskMetricsCollector::default())).unwrap();
            prometheus::register(Box::new(LinuxProcessMetricsCollector::default())).unwrap();
        }
    }
}
