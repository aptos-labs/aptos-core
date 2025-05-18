// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::collectors::BasicNodeInfoCollector;
use aptos_infallible::Mutex;
use aptos_logger::warn;
use cfg_if::cfg_if;
use collectors::{
    CollectorLatencyCollector, CpuMetricsCollector, DiskMetricsCollector, LoadAvgCollector,
    MemoryMetricsCollector, NetworkMetricsCollector, ProcessMetricsCollector,
};
use once_cell::sync::Lazy;
use prometheus::core::Collector;
use std::collections::BTreeMap;

mod collectors;

static IS_REGISTERED: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

/// Registers the node metrics collector with the default registry.
pub fn register_node_metrics_collector(maybe_build_info: Option<&BTreeMap<String, String>>) {
    let mut registered = IS_REGISTERED.lock();
    if *registered {
        return;
    } else {
        *registered = true;
    }

    register_collector(Box::<CpuMetricsCollector>::default());
    register_collector(Box::<MemoryMetricsCollector>::default());
    register_collector(Box::<DiskMetricsCollector>::default());
    register_collector(Box::<NetworkMetricsCollector>::default());
    register_collector(Box::<LoadAvgCollector>::default());
    register_collector(Box::<ProcessMetricsCollector>::default());
    register_collector(Box::new(BasicNodeInfoCollector::new(maybe_build_info)));
    cfg_if! {
        if #[cfg(all(target_os="linux"))] {
            register_collector(Box::<collectors::LinuxCpuMetricsCollector>::default());
            register_collector(Box::<collectors::LinuxDiskMetricsCollector>::default());
        }
    }
    register_collector(Box::<CollectorLatencyCollector>::default());
}

pub fn register_collector(c: Box<dyn Collector>) {
    // If not okay, then log the error and continue.
    prometheus::register(c).unwrap_or_else(|e| {
        warn!("Failed to register collector: {}", e);
    });
}
