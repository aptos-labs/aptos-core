// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod common;
mod cpu_metrics_collector;
mod disk_metrics_collector;
mod loadavg_collector;
mod memory_metrics_collector;
mod network_metrics_collector;
mod process_metrics_collector;

pub(crate) use common::CollectorLatencyCollector;
pub(crate) use cpu_metrics_collector::CpuMetricsCollector;
pub(crate) use disk_metrics_collector::DiskMetricsCollector;
pub(crate) use loadavg_collector::LoadAvgCollector;
pub(crate) use memory_metrics_collector::MemoryMetricsCollector;
pub(crate) use network_metrics_collector::NetworkMetricsCollector;
pub(crate) use process_metrics_collector::ProcessMetricsCollector;

#[cfg(all(target_os = "linux"))]
mod linux_collectors;
#[cfg(all(target_os = "linux"))]
pub(crate) use linux_collectors::LinuxCpuMetricsCollector;
#[cfg(all(target_os = "linux"))]
pub(crate) use linux_collectors::LinuxDiskMetricsCollector;
