// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use collectors::{
    CpuCollector, DiskCollector, LoadAvgCollector, MemoryCollector, NetworkCollector,
    ProcessCollector,
};

mod collectors;

/// Registers the node metrics collector with the default registry.
pub fn register_node_metrics_collector() {
    prometheus::register(Box::new(CpuCollector::default())).unwrap();
    prometheus::register(Box::new(MemoryCollector::default())).unwrap();
    prometheus::register(Box::new(DiskCollector::default())).unwrap();
    prometheus::register(Box::new(NetworkCollector::default())).unwrap();
    prometheus::register(Box::new(LoadAvgCollector::default())).unwrap();
    prometheus::register(Box::new(ProcessCollector::default())).unwrap();
}
