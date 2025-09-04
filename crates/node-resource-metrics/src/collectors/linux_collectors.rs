// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::common::NAMESPACE;
use crate::collectors::common::MeasureLatency;
use velor_logger::warn;
use velor_metrics_core::const_metric::ConstMetric;
use procfs::{DiskStat, KernelStats};
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::collections::HashMap;

const LINUX_SYSTEM_CPU_USAGE: &str = "linux_system_cpu_usage";
const LINUX_CPU_METRICS_COUNT: usize = 11;
const LINUX_CPU_STATE_LABEL: &str = "state";

const LINUX_DISK_SUBSYSTEM: &str = "linux_disk";
const NAME_LABEL: &str = "name";
/// The following are the fields as reported by `/proc/diskstats` in Linux.
/// More details on each of these fields are here: https://www.kernel.org/doc/Documentation/iostats.txt
const LINUX_NUM_READS: &str = "num_reads";
const LINUX_NUM_MERGED_READS: &str = "num_merged_reads";
const LINUX_NUM_SECTORS_READ: &str = "num_sectors_read";
const LINUX_TIME_READING_MS: &str = "time_reading_ms";
const LINUX_NUM_WRITES: &str = "num_writes";
const LINUX_NUM_MERGED_WRITES: &str = "num_merged_writes";
const LINUX_NUM_SECTORS_WRITTEN: &str = "num_sectors_written";
const LINUX_TIME_WRITING_MS: &str = "time_writing_ms";
const LINUX_PROGRESS_IO: &str = "io_in_progress";
const LINUX_TOTAL_IO_TIME_MS: &str = "total_io_time_ms";

const LINUX_RLIMIT_NOFILE_SOFT: &str = "rlimit_nofile_soft";

const LINUX_RLIMIT_NOFILE_HARD: &str = "rlimit_nofile_hard";

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
        cpu_time_counter!(mfs, kernal_stats.ctxt, "context_switches");

        mfs
    }
}

/// A Collector for exposing Linux Disk stats as metrics
pub(crate) struct LinuxDiskMetricsCollector {
    num_reads: Desc,
    num_merged_reads: Desc,
    num_sectors_read: Desc,
    time_reading_ms: Desc,
    num_writes: Desc,
    num_merged_writes: Desc,
    num_sectors_written: Desc,
    time_writing_ms: Desc,
    io_in_progress: Desc,
    total_io_time_ms: Desc,
    rlimit_nofile_soft: ConstMetric,
    rlimit_nofile_hard: ConstMetric,
}

impl LinuxDiskMetricsCollector {
    fn new() -> Self {
        macro_rules! disk_desc {
            ($NAME:ident, $HELP:expr) => {
                Opts::new($NAME, $HELP)
                    .namespace(NAMESPACE)
                    .subsystem(LINUX_DISK_SUBSYSTEM)
                    .variable_labels(vec![NAME_LABEL.into()])
                    .describe()
                    .unwrap()
            };
        }

        let (soft, hard) = rlimit::Resource::NOFILE.get().unwrap_or((0, 0));
        Self {
            num_reads: disk_desc!(
                LINUX_NUM_READS,
                "Total number of reads completed successfully"
            ),
            num_merged_reads: disk_desc!(
                LINUX_NUM_MERGED_READS,
                "Total number of adjacent merged reads"
            ),
            num_sectors_read: disk_desc!(
                LINUX_NUM_SECTORS_READ,
                "Total number of sectors read successfully"
            ),
            time_reading_ms: disk_desc!(
                LINUX_TIME_READING_MS,
                "Total number of milliseconds spent by all reads "
            ),
            num_writes: disk_desc!(
                LINUX_NUM_WRITES,
                "Total number of writes completed successfully"
            ),
            num_merged_writes: disk_desc!(
                LINUX_NUM_MERGED_WRITES,
                "Total number of adjacent merged writes"
            ),
            num_sectors_written: disk_desc!(
                LINUX_NUM_SECTORS_WRITTEN,
                "Total number of sectors written successfully"
            ),
            time_writing_ms: disk_desc!(
                LINUX_TIME_WRITING_MS,
                "Total number of milliseconds spend by all writes"
            ),
            io_in_progress: disk_desc!(LINUX_PROGRESS_IO, "Number of IOs in progress"),
            total_io_time_ms: disk_desc!(
                LINUX_TOTAL_IO_TIME_MS,
                "Total number of milliseconds spent in IO"
            ),
            rlimit_nofile_soft: ConstMetric::new_gauge(
                Opts::new(LINUX_RLIMIT_NOFILE_SOFT, "RLIMIT_NOFILE soft limit.")
                    .namespace(NAMESPACE)
                    .subsystem(LINUX_DISK_SUBSYSTEM)
                    .describe()
                    .unwrap(),
                soft as f64,
                None,
            )
            .unwrap(),
            rlimit_nofile_hard: ConstMetric::new_gauge(
                Opts::new(LINUX_RLIMIT_NOFILE_HARD, "RLIMIT_NOFILE hard limit.")
                    .namespace(NAMESPACE)
                    .subsystem(LINUX_DISK_SUBSYSTEM)
                    .describe()
                    .unwrap(),
                hard as f64,
                None,
            )
            .unwrap(),
        }
    }
}

impl Default for LinuxDiskMetricsCollector {
    fn default() -> Self {
        LinuxDiskMetricsCollector::new()
    }
}

impl Collector for LinuxDiskMetricsCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![
            &self.num_reads,
            &self.num_merged_reads,
            &self.num_sectors_read,
            &self.time_reading_ms,
            &self.num_writes,
            &self.num_merged_writes,
            &self.num_sectors_written,
            &self.time_writing_ms,
            &self.io_in_progress,
            &self.total_io_time_ms,
        ]
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let _measure = MeasureLatency::new("linux_disk".into());

        macro_rules! disk_stats_counter {
            ($METRICS:ident, $DESC:ident, $FIELD:expr, $LABELS:expr) => {
                $METRICS.extend(
                    ConstMetric::new_counter(self.$DESC.clone(), $FIELD as f64, Some($LABELS))
                        .unwrap()
                        .collect(),
                );
            };
        }
        macro_rules! disk_stats_guage {
            ($METRICS:ident, $DESC:ident, $FIELD:expr, $LABELS:expr) => {
                $METRICS.extend(
                    ConstMetric::new_gauge(self.$DESC.clone(), $FIELD as f64, Some($LABELS))
                        .unwrap()
                        .collect(),
                );
            };
        }

        let mut mfs = Vec::new();
        mfs.extend(self.rlimit_nofile_soft.collect());
        mfs.extend(self.rlimit_nofile_hard.collect());

        let disk_stats: HashMap<String, DiskStat> = match procfs::diskstats() {
            Ok(disk_stats) => HashMap::from_iter(disk_stats.into_iter().filter_map(|stat| {
                if !stat.name.starts_with("loop") {
                    Some((format!("{}:{}", stat.major, stat.minor), stat))
                } else {
                    None
                }
            })),
            Err(err) => {
                warn!("unable to collect disk metrics for linux: {}", err);
                return mfs;
            },
        };

        let mounts =
            match procfs::process::Process::myself().and_then(|process| process.mountinfo()) {
                Ok(mounts) => mounts,
                Err(err) => {
                    warn!(
                    "unable to collect disk metrics for linux. failure collecting mountinfo: {}",
                    err
                );
                    return mfs;
                },
            };

        for mount in mounts {
            if let Some(disk_stat) = disk_stats.get(&mount.majmin) {
                let labels = &[disk_stat.name.clone()];
                disk_stats_counter!(mfs, num_reads, disk_stat.reads, labels);
                disk_stats_counter!(mfs, num_merged_reads, disk_stat.merged, labels);
                disk_stats_counter!(mfs, num_sectors_read, disk_stat.sectors_read, labels);
                disk_stats_counter!(mfs, time_reading_ms, disk_stat.time_reading, labels);
                disk_stats_counter!(mfs, num_writes, disk_stat.writes, labels);
                disk_stats_counter!(mfs, num_merged_writes, disk_stat.writes_merged, labels);
                disk_stats_counter!(mfs, num_sectors_written, disk_stat.sectors_written, labels);
                disk_stats_counter!(mfs, time_writing_ms, disk_stat.time_writing, labels);
                disk_stats_guage!(mfs, io_in_progress, disk_stat.in_progress, labels);
                disk_stats_counter!(mfs, total_io_time_ms, disk_stat.time_in_progress, labels);
            }
        }

        mfs
    }
}

#[cfg(test)]
mod tests {
    use super::{LinuxCpuMetricsCollector, LinuxDiskMetricsCollector};
    use prometheus::Registry;

    #[test]
    fn test_linux_cpu_collector_register() {
        let collector = LinuxCpuMetricsCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }

    #[test]
    fn test_linux_disk_collector_register() {
        let collector = LinuxDiskMetricsCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
