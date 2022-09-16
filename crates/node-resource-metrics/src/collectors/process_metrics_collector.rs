// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::Mutex;
use aptos_logger::warn;
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::sync::Arc;
use sysinfo::{ProcessExt, System, SystemExt};

use super::common::NAMESPACE;

const PROCESS_METRICS_COUNT: usize = 3;
const PROCESS_SUBSYSTEM: &str = "process";

const MEMORY: &str = "memory";
const VIRTUAL_MEMORY: &str = "virtual_memory";
const START_TIME: &str = "start_time";
const RUN_TIME: &str = "run_time";
const CPU_USAGE: &str = "cpu_usage";
const TOTAL_READ_BYTES: &str = "disk_total_read_bytes";
const TOTAL_WRITTEN_BYTES: &str = "disk_total_written_bytes";

const LINUX_PROCESS_SUBSYSTEM: &str = "linux_process";
const LINUX_PROCESS_METRICS_COUNT: usize = 2;

const LINUX_VMEM_SIZE: &str = "vm_size_bytes";
const LINUX_VMEM_RSS: &str = "vm_rss_bytes";

/// A Collector for exposing process metrics
pub(crate) struct ProcessMetricsCollector {
    system: Arc<Mutex<System>>,

    memory: Desc,
    virtual_memory: Desc,
    start_time: Desc,
    run_time: Desc,
    cpu_usage: Desc,
    total_read_bytes: Desc,
    total_written_bytes: Desc,
}

impl ProcessMetricsCollector {
    fn new() -> Self {
        let system = Arc::new(Mutex::new(System::new()));

        let memory = Opts::new(MEMORY, "Memory usage in bytes.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let virtual_memory = Opts::new(VIRTUAL_MEMORY, "Virtual memory usage in bytes.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let start_time = Opts::new(
            START_TIME,
            "Starts time of the process in seconds since epoch.",
        )
        .namespace(NAMESPACE)
        .subsystem(PROCESS_SUBSYSTEM)
        .describe()
        .unwrap();
        let run_time = Opts::new(RUN_TIME, "Run time of the process in seconds.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let cpu_usage = Opts::new(CPU_USAGE, "CPU usage.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let total_read_bytes = Opts::new(TOTAL_READ_BYTES, "Total bytes read.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let total_written_bytes = Opts::new(TOTAL_WRITTEN_BYTES, "Total bytes written.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();

        Self {
            system,
            memory,
            virtual_memory,
            start_time,
            run_time,
            cpu_usage,
            total_read_bytes,
            total_written_bytes,
        }
    }
}

impl Default for ProcessMetricsCollector {
    fn default() -> Self {
        ProcessMetricsCollector::new()
    }
}

impl Collector for ProcessMetricsCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![
            &self.memory,
            &self.virtual_memory,
            &self.start_time,
            &self.run_time,
            &self.cpu_usage,
            &self.total_read_bytes,
            &self.total_written_bytes,
        ]
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let mut system = self.system.lock();

        let pid = if let Ok(pid) = sysinfo::get_current_pid() {
            system.refresh_process(pid);
            pid
        } else {
            return Vec::new();
        };

        let process = system.process(pid);
        let process = if let Some(process) = process {
            process
        } else {
            return Vec::new();
        };

        let memory =
            ConstMetric::new_gauge(self.memory.clone(), process.memory() as f64, None).unwrap();

        let virtual_memory = ConstMetric::new_gauge(
            self.virtual_memory.clone(),
            process.virtual_memory() as f64,
            None,
        )
        .unwrap();

        let start_time =
            ConstMetric::new_gauge(self.start_time.clone(), process.start_time() as f64, None)
                .unwrap();

        let run_time =
            ConstMetric::new_gauge(self.run_time.clone(), process.run_time() as f64, None).unwrap();

        let cpu_usage =
            ConstMetric::new_gauge(self.cpu_usage.clone(), process.cpu_usage() as f64, None)
                .unwrap();

        let total_read_bytes = ConstMetric::new_gauge(
            self.total_read_bytes.clone(),
            process.disk_usage().total_read_bytes as f64,
            None,
        )
        .unwrap();

        let total_written_bytes = ConstMetric::new_gauge(
            self.total_written_bytes.clone(),
            process.disk_usage().total_written_bytes as f64,
            None,
        )
        .unwrap();

        let mut mfs = Vec::with_capacity(PROCESS_METRICS_COUNT);
        mfs.extend(memory.collect());
        mfs.extend(virtual_memory.collect());
        mfs.extend(start_time.collect());
        mfs.extend(run_time.collect());
        mfs.extend(cpu_usage.collect());
        mfs.extend(total_read_bytes.collect());
        mfs.extend(total_written_bytes.collect());

        mfs
    }
}

pub(crate) struct LinuxProcessMetricsCollector {
    vm_size_bytes: Desc,
    vm_rss_bytes: Desc,
}

impl LinuxProcessMetricsCollector {
    fn new() -> Self {
        let vm_size_bytes = Opts::new(LINUX_VMEM_SIZE, "Memory usage in bytes.")
            .namespace(NAMESPACE)
            .subsystem(LINUX_PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let vm_rss_bytes = Opts::new(LINUX_VMEM_RSS, "Memory usage in bytes.")
            .namespace(NAMESPACE)
            .subsystem(LINUX_PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        Self {
            vm_size_bytes,
            vm_rss_bytes,
        }
    }
}

impl Default for LinuxProcessMetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for LinuxProcessMetricsCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![&self.vm_size_bytes, &self.vm_rss_bytes]
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let mut mfs = Vec::with_capacity(LINUX_PROCESS_METRICS_COUNT);

        let page_size = match procfs::page_size() {
            Ok(page_size) => page_size,
            Err(err) => {
                warn!(
                    "unable to get page_size in linux, not collecting process memory metrics: {}",
                    err
                );
                return mfs;
            }
        };

        let proc_statm = procfs::process::Process::myself().and_then(|p| p.statm());
        if proc_statm.is_err() {
            warn!(
                "unable to collect process memory metrics for linux: {}",
                proc_statm.unwrap_err()
            );
            return mfs;
        }

        let proc_statm = proc_statm.unwrap();
        let vm_size_bytes = ConstMetric::new_gauge(
            self.vm_size_bytes.clone(),
            (proc_statm.size * page_size) as f64,
            None,
        )
        .unwrap();
        let vm_rss_bytes = ConstMetric::new_gauge(
            self.vm_rss_bytes.clone(),
            (proc_statm.resident * page_size) as f64,
            None,
        )
        .unwrap();

        mfs.extend(vm_size_bytes.collect());
        mfs.extend(vm_rss_bytes.collect());

        mfs
    }
}

#[cfg(test)]
mod tests {
    use super::{LinuxProcessMetricsCollector, ProcessMetricsCollector};
    use prometheus::Registry;

    #[test]
    fn test_process_collector_register() {
        let collector = ProcessMetricsCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }

    #[test]
    fn test_linux_process_collector_register() {
        let collector = LinuxProcessMetricsCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
