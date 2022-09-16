// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use collectors::{
    CpuMetricsCollector, DiskMetricsCollector, LoadAvgCollector, MemoryMetricsCollector,
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
}
