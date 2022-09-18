// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::common::NAMESPACE;
use crate::collectors::common::MeasureLatency;
use aptos_infallible::Mutex;
use aptos_logger::warn;
use aptos_metrics_core::const_metric::ConstMetric;
use procfs::KernelStats;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::sync::Arc;
use sysinfo::{CpuExt, CpuRefreshKind, RefreshKind, System, SystemExt};

const SYSTEM_CPU_USAGE: &str = "system_cpu_usage";
const SYSTEM_CPU_INFO: &str = "system_cpu_info";

const CPU_ID_LABEL: &str = "cpu_id";
const CPU_BRAND_LABEL: &str = "brand";
const CPU_VENDOR_LABEL: &str = "vendor";

const LINUX_SYSTEM_CPU_USAGE: &str = "linux_system_cpu_usage";

const LINUX_CPU_METRICS_COUNT: usize = 10;
const LINUX_CPU_STATE_LABEL: &str = "state";

/// A Collector for exposing CPU metrics
pub(crate) struct CpuMetricsCollector {
    system: Arc<Mutex<System>>,

    cpu: Desc,
    cpu_info: Desc,
}

impl CpuMetricsCollector {
    fn new() -> Self {
        let system = Arc::new(Mutex::new(System::new_with_specifics(
            RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
        )));

        let cpu = Opts::new(SYSTEM_CPU_USAGE, "CPU usage.")
            .namespace(NAMESPACE)
            .variable_label(CPU_ID_LABEL)
            .describe()
            .unwrap();
        let cpu_info = Opts::new(SYSTEM_CPU_INFO, "CPU information.")
            .namespace(NAMESPACE)
            .variable_label(CPU_BRAND_LABEL)
            .variable_label(CPU_VENDOR_LABEL)
            .describe()
            .unwrap();

        Self {
            system,
            cpu,
            cpu_info,
        }
    }
}

impl Default for CpuMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for CpuMetricsCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![&self.cpu, &self.cpu_info]
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let _measure = MeasureLatency::new("cpu".into());

        let mut system = self.system.lock();

        system.refresh_cpu();

        let global_cpu_info = &system.global_cpu_info();
        let cpu_usage = ConstMetric::new_gauge(
            self.cpu.clone(),
            global_cpu_info.cpu_usage() as f64,
            Some(&[system.global_cpu_info().name().into()]),
        )
        .unwrap();

        let per_cpu_usage: Vec<MetricFamily> = system
            .cpus()
            .iter()
            .enumerate()
            .flat_map(|(idx, cpu)| {
                ConstMetric::new_gauge(
                    self.cpu.clone(),
                    cpu.cpu_usage() as f64,
                    Some(&[format!("cpu{}_idx{}", cpu.name(), idx + 1)]),
                )
                .unwrap()
                .collect()
            })
            .collect();

        let cpu_info = ConstMetric::new_gauge(
            self.cpu_info.clone(),
            1.0,
            Some(&[
                global_cpu_info.brand().into(),
                global_cpu_info.vendor_id().into(),
            ]),
        )
        .unwrap();

        let mut mfs = Vec::with_capacity(2 + per_cpu_usage.len());
        mfs.extend(cpu_usage.collect());
        mfs.extend(per_cpu_usage);
        mfs.extend(cpu_info.collect());

        mfs
    }
}

/// A Collector for exposing Linux CPU metrics
pub(crate) struct LinuxCpuMetricsCollector {
    cpu: Desc,
}

impl LinuxCpuMetricsCollector {
    fn new() -> Self {
        let cpu = Opts::new(LINUX_SYSTEM_CPU_USAGE, "Linux CPU usage.")
            .namespace(NAMESPACE)
            .variable_label(LINUX_CPU_STATE_LABEL)
            .describe()
            .unwrap();

        Self { cpu }
    }
}

impl Default for LinuxCpuMetricsCollector {
    fn default() -> Self {
        LinuxCpuMetricsCollector::new()
    }
}

impl Collector for LinuxCpuMetricsCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![&self.cpu]
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let _measure = MeasureLatency::new("linux_cpu".into());

        macro_rules! cpu_time_counter {
            ($METRICS:ident, $FIELD:expr, $LABEL:expr) => {
                $METRICS.extend(
                    ConstMetric::new_counter(
                        self.cpu.clone(),
                        $FIELD as f64,
                        Some(&[$LABEL.into()]),
                    )
                    .unwrap()
                    .collect(),
                );
            };
        }
        macro_rules! cpu_time_counter_opt {
            ($METRICS:ident, $OPT_FIELD:expr, $LABEL:expr) => {
                if let Some(field) = $OPT_FIELD {
                    cpu_time_counter!($METRICS, field, $LABEL);
                }
            };
        }

        let mut mfs = Vec::with_capacity(LINUX_CPU_METRICS_COUNT);

        let kernel_stats = KernelStats::new();
        if kernel_stats.is_err() {
            warn!(
                "unable to collect cpu metrics for linux: {}",
                kernel_stats.unwrap_err()
            );
            return mfs;
        }

        let kernal_stats = kernel_stats.unwrap();
        let cpu_time = kernal_stats.total;

        cpu_time_counter!(mfs, cpu_time.user_ms(), "user_ms");
        cpu_time_counter!(mfs, cpu_time.nice_ms(), "nice_ms");
        cpu_time_counter!(mfs, cpu_time.system_ms(), "system_ms");
        cpu_time_counter!(mfs, cpu_time.idle_ms(), "idle_ms");
        cpu_time_counter_opt!(mfs, cpu_time.iowait_ms(), "iowait_ms");
        cpu_time_counter_opt!(mfs, cpu_time.irq_ms(), "irq_ms");
        cpu_time_counter_opt!(mfs, cpu_time.softirq_ms(), "softirq_ms");
        cpu_time_counter_opt!(mfs, cpu_time.steal_ms(), "steal_ms");
        cpu_time_counter_opt!(mfs, cpu_time.guest_ms(), "guest_ms");
        cpu_time_counter_opt!(mfs, cpu_time.guest_nice_ms(), "guest_nice_ms");

        mfs
    }
}

#[cfg(test)]
mod tests {
    use super::{CpuMetricsCollector, LinuxCpuMetricsCollector};
    use prometheus::Registry;

    #[test]
    fn test_cpu_collector_register() {
        let collector = CpuMetricsCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }

    #[test]
    fn test_linux_cpu_collector_register() {
        let collector = LinuxCpuMetricsCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
