// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::collectors::BasicNodeInfoCollector;
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

    prometheus::register(Box::<CpuMetricsCollector>::default()).unwrap();
    prometheus::register(Box::<MemoryMetricsCollector>::default()).unwrap();
    prometheus::register(Box::<DiskMetricsCollector>::default()).unwrap();
    prometheus::register(Box::<NetworkMetricsCollector>::default()).unwrap();
    prometheus::register(Box::<LoadAvgCollector>::default()).unwrap();
    prometheus::register(Box::<ProcessMetricsCollector>::default()).unwrap();
    prometheus::register(Box::<BasicNodeInfoCollector>::default()).unwrap();
    cfg_if! {
        if #[cfg(all(target_os="linux"))] {
            prometheus::register(Box::<collectors::LinuxCpuMetricsCollector>::default()).unwrap();
            prometheus::register(Box::<collectors::LinuxDiskMetricsCollector>::default()).unwrap();
        }
    }
    prometheus::register(Box::<CollectorLatencyCollector>::default()).unwrap();
}
