// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::collectors::common::{MeasureLatency, NAMESPACE};
use aptos_infallible::Mutex;
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::sync::Arc;
use sysinfo::{ProcessExt, System, SystemExt};

const PROCESS_METRICS_COUNT: usize = 3;
const PROCESS_SUBSYSTEM: &str = "process";

const MEMORY: &str = "memory";
const VIRTUAL_MEMORY: &str = "virtual_memory";
const START_TIME: &str = "start_time";
const RUN_TIME: &str = "run_time";
const CPU_USAGE: &str = "cpu_usage";
const TOTAL_READ_BYTES: &str = "disk_total_read_bytes";
const TOTAL_WRITTEN_BYTES: &str = "disk_total_written_bytes";

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

        let memory = Opts::new(MEMORY, "Total memory usage (rss) in bytes.")
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
        let _measure = MeasureLatency::new("process".into());

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
    use super::ProcessMetricsCollector;
    use prometheus::Registry;

    #[test]
    fn test_process_collector_register() {
        let collector = ProcessMetricsCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
