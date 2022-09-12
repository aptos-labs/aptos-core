// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::common::NAMESPACE;
use aptos_infallible::Mutex;
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::sync::Arc;
use sysinfo::{CpuExt, CpuRefreshKind, RefreshKind, System, SystemExt};

/// A Collector for exposing CPU metrics
pub(crate) struct CpuCollector {
    system: Arc<Mutex<System>>,

    cpu: Desc,
    cpu_info: Desc,
}

impl CpuCollector {
    fn new() -> Self {
        let system = Arc::new(Mutex::new(System::new_with_specifics(
            RefreshKind::new().with_cpu(CpuRefreshKind::everything()),
        )));

        let cpu = Opts::new("system_cpu_usage", "CPU usage.")
            .namespace(NAMESPACE)
            .variable_label("cpu_id")
            .describe()
            .unwrap();
        let cpu_info = Opts::new("system_cpu_info", "CPU information.")
            .namespace(NAMESPACE)
            .variable_label("brand")
            .variable_label("vendor")
            .describe()
            .unwrap();

        Self {
            system,
            cpu,
            cpu_info,
        }
    }
}

impl Default for CpuCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for CpuCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![&self.cpu, &self.cpu_info]
    }

    fn collect(&self) -> Vec<MetricFamily> {
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

#[cfg(test)]
mod tests {
    use super::CpuCollector;
    use prometheus::Registry;

    #[test]
    fn test_cpu_collector_register() {
        let collector = CpuCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
