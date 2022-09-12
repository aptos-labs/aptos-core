// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use aptos_infallible::Mutex;
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use sysinfo::{ProcessExt, System, SystemExt};

use super::common::NAMESPACE;

const PROCESS_METRICS_COUNT: usize = 3;
const PROCESS_SUBSYSTEM: &str = "process";

/// A Collector for exposing process metrics
pub(crate) struct ProcessCollector {
    system: Arc<Mutex<System>>,

    memory: Desc,
    virtual_memory: Desc,
    start_time: Desc,
    run_time: Desc,
    cpu_usage: Desc,
    total_read_bytes: Desc,
    total_written_bytes: Desc,
}

impl ProcessCollector {
    fn new() -> Self {
        let system = Arc::new(Mutex::new(System::new()));

        let memory = Opts::new("memory", "Memory usage in bytes.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let virtual_memory = Opts::new("virtual_memory", "Virtual memory usage in bytes.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let start_time = Opts::new(
            "start_time",
            "Starts time of the process in seconds since epoch.",
        )
        .namespace(NAMESPACE)
        .subsystem(PROCESS_SUBSYSTEM)
        .describe()
        .unwrap();
        let run_time = Opts::new("run_time", "Run time of the process in seconds.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let cpu_usage = Opts::new("cpu_usage", "CPU usage.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let total_read_bytes = Opts::new("disk_total_read_bytes", "Total bytes read.")
            .namespace(NAMESPACE)
            .subsystem(PROCESS_SUBSYSTEM)
            .describe()
            .unwrap();
        let total_written_bytes = Opts::new("disk_total_written_bytes", "Total bytes written.")
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

impl Default for ProcessCollector {
    fn default() -> Self {
        ProcessCollector::new()
    }
}

impl Collector for ProcessCollector {
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

#[cfg(test)]
mod tests {
    use super::ProcessCollector;
    use prometheus::Registry;

    #[test]
    fn test_cpu_collector_register() {
        let collector = ProcessCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
