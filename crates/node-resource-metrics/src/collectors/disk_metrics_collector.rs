// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::common::NAMESPACE;
use crate::collectors::common::MeasureLatency;
use aptos_infallible::Mutex;
use aptos_logger::warn;
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::sync::Arc;
use sysinfo::{DiskExt, RefreshKind, System, SystemExt};

const DISK_SUBSYSTEM: &str = "disk";

const TOTAL_SPACE: &str = "total_space";
const AVAILABLE_SPACE: &str = "available_space";

const NAME_LABEL: &str = "name";
const TYPE_LABEL: &str = "type";
const FILE_SYSTEM_LABEL: &str = "file_system";

const LINUX_DISK_SUBSYSTEM: &str = "linux_disk";

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

/// A Collector for exposing Disk metrics
pub(crate) struct DiskMetricsCollector {
    system: Arc<Mutex<System>>,

    total_space: Desc,
    available_space: Desc,
}

impl DiskMetricsCollector {
    fn new() -> Self {
        let system = Arc::new(Mutex::new(System::new_with_specifics(
            RefreshKind::new().with_disks_list().with_disks(),
        )));
        let total_space = Opts::new(TOTAL_SPACE, "Total disk size in bytes")
            .namespace(NAMESPACE)
            .subsystem(DISK_SUBSYSTEM)
            .variable_labels(vec![
                NAME_LABEL.into(),
                TYPE_LABEL.into(),
                FILE_SYSTEM_LABEL.into(),
            ])
            .describe()
            .unwrap();
        let available_space = Opts::new(AVAILABLE_SPACE, "Total available disk size in bytes")
            .namespace(NAMESPACE)
            .subsystem(DISK_SUBSYSTEM)
            .variable_labels(vec![
                NAME_LABEL.into(),
                TYPE_LABEL.into(),
                FILE_SYSTEM_LABEL.into(),
            ])
            .describe()
            .unwrap();

        Self {
            system,
            total_space,
            available_space,
        }
    }
}

impl Default for DiskMetricsCollector {
    fn default() -> Self {
        DiskMetricsCollector::new()
    }
}

impl Collector for DiskMetricsCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![&self.total_space, &self.available_space]
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let _measure = MeasureLatency::new("disk".into());

        let mut system = self.system.lock();
        system.refresh_disks_list();
        system.refresh_disks();

        let mfs = system
            .disks()
            .iter()
            .flat_map(|disk| {
                let total_space = ConstMetric::new_counter(
                    self.total_space.clone(),
                    disk.total_space() as f64,
                    Some(&[
                        disk.name().to_string_lossy().into_owned(),
                        format!("{:?}", disk.type_()),
                        String::from_utf8_lossy(disk.file_system()).to_string(),
                    ]),
                )
                .unwrap();
                let available_space = ConstMetric::new_counter(
                    self.available_space.clone(),
                    disk.available_space() as f64,
                    Some(&[
                        disk.name().to_string_lossy().into_owned(),
                        format!("{:?}", disk.type_()),
                        String::from_utf8_lossy(disk.file_system()).to_string(),
                    ]),
                )
                .unwrap();

                vec![total_space, available_space]
            })
            .flat_map(|metric| metric.collect())
            .collect();

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

        let disk_stats = match procfs::diskstats() {
            Ok(disk_stats) => disk_stats,
            Err(err) => {
                warn!("unable to collect disk metrics for linux: {}", err);
                return mfs;
            }
        };

        disk_stats
            .into_iter()
            .filter(|disk_stat| disk_stat.name.starts_with("sd"))
            .for_each(|disk_stat| {
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
            });

        mfs
    }
}

#[cfg(test)]
mod tests {
    use super::{DiskMetricsCollector, LinuxDiskMetricsCollector};
    use prometheus::Registry;

    #[test]
    fn test_disk_collector_register() {
        let collector = DiskMetricsCollector::default();

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
