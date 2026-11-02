// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::common::NAMESPACE;
use crate::collectors::common::MeasureLatency;
use aptos_infallible::Mutex;
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::sync::Arc;
use sysinfo::{CpuRefreshKind, RefreshKind, System};

const SYSTEM_CPU_USAGE: &str = "system_cpu_usage";
const SYSTEM_CPU_INFO: &str = "system_cpu_info";

/// The global CPU name that older sysinfo versions reported on Linux, kept for metric continuity.
const GLOBAL_CPU_ID: &str = "cpu";

const CPU_ID_LABEL: &str = "cpu_id";
const CPU_BRAND_LABEL: &str = "brand";
const CPU_VENDOR_LABEL: &str = "vendor";

/// A Collector for exposing CPU metrics
pub(crate) struct CpuMetricsCollector {
    system: Arc<Mutex<System>>,
    cpu: Desc,
    cpu_info: Desc,
}

impl CpuMetricsCollector {
    fn new() -> Self {
        let system = Arc::new(Mutex::new(System::new_with_specifics(
            RefreshKind::nothing().with_cpu(CpuRefreshKind::nothing().with_cpu_usage()),
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

        system.refresh_cpu_usage();

        let cpu_usage = ConstMetric::new_gauge(
            self.cpu.clone(),
            system.global_cpu_usage() as f64,
            Some(&[GLOBAL_CPU_ID.into()]),
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

        let (brand, vendor_id) = system
            .cpus()
            .first()
            .map(|cpu| (cpu.brand(), cpu.vendor_id()))
            .unwrap_or_default();
        let cpu_info = ConstMetric::new_gauge(
            self.cpu_info.clone(),
            1.0,
            Some(&[brand.into(), vendor_id.into()]),
        )
        .unwrap();

        let mut mfs = Vec::with_capacity(2 + per_cpu_usage.len());
        mfs.extend(cpu_usage.collect());
        mfs.extend(per_cpu_usage);
        mfs.extend(cpu_info.collect());

        mfs
    }
}

#[cfg(test)]
mod tests {
    use super::CpuMetricsCollector;
    use prometheus::Registry;

    #[test]
    fn test_cpu_collector_register() {
        let collector = CpuMetricsCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
