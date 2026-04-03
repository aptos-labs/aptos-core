// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::common::NAMESPACE;
use crate::collectors::common::MeasureLatency;
use aptos_logger::warn;
use aptos_metrics_core::const_metric::ConstMetric;
use procfs::{DiskStat, KernelStats};
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::collections::{HashMap, HashSet};

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

const LINUX_DISK_INFO: &str = "info";
const MODEL_LABEL: &str = "model";
const RAID_LEVEL_LABEL: &str = "raid_level";

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

        let kernel_stats = match KernelStats::new() {
            Ok(stats) => stats,
            Err(err) => {
                warn!("unable to collect cpu metrics for linux: {}", err);
                return mfs;
            },
        };
        let cpu_time = kernel_stats.total;

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
        cpu_time_counter!(mfs, kernel_stats.ctxt, "context_switches");

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
    disk_info: Desc,
    /// Cached disk info: device name -> (model, raid_level)
    disk_info_cache: HashMap<String, (String, String)>,
    rlimit_nofile_soft: ConstMetric,
    rlimit_nofile_hard: ConstMetric,
}

/// Resolve the drive model for a block device by walking sysfs.
///
/// For a partition (e.g. `sda1`, `nvme0n1p1`), `/sys/block/<part>/device/model`
/// does not exist — only whole-disk entries have it. We use
/// `/sys/class/block/<device>` (a symlink into the device tree) and walk up to
/// the nearest ancestor that has a `device/model` file. This handles both
/// SCSI (`sda1` → `sda`) and NVMe (`nvme0n1p1` → `nvme0n1`) naming correctly
/// without string manipulation.
///
/// For md/RAID devices, the same resolution is applied to the first slave.
fn read_disk_model(device: &str) -> String {
    // Walk up from /sys/class/block/<device> to find the nearest device/model.
    if let Some(model) = resolve_model_via_sysfs(device) {
        return model;
    }
    // For md/RAID devices, try slaves sorted alphabetically for determinism
    // (read_dir order is filesystem-dependent and can vary across restarts).
    if let Ok(entries) = std::fs::read_dir(format!("/sys/block/{}/slaves", device)) {
        let mut slaves: Vec<String> = entries
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        slaves.sort();
        for slave in &slaves {
            if let Some(model) = resolve_model_via_sysfs(slave) {
                return model;
            }
        }
    }
    "unknown".to_string()
}

/// Walk from `/sys/class/block/<device>` up the directory tree looking for
/// `device/model`. Returns `None` if no ancestor has it.
fn resolve_model_via_sysfs(device: &str) -> Option<String> {
    let resolved = std::fs::canonicalize(format!("/sys/class/block/{}", device)).ok()?;
    let mut path = resolved.as_path();
    loop {
        let model_path = path.join("device/model");
        if let Ok(model) = std::fs::read_to_string(&model_path) {
            return Some(model.trim().to_string());
        }
        path = path.parent()?;
        // Stop before walking above /sys
        if path.as_os_str() == "/sys" || path.as_os_str() == "/" {
            return None;
        }
    }
}

fn read_raid_level(device: &str) -> String {
    std::fs::read_to_string(format!("/sys/block/{}/md/level", device))
        .map(|s| s.trim().to_string())
        .unwrap_or_default()
}

/// Build a cache of (model, raid_level) for all non-loop block devices.
fn build_disk_info_cache() -> HashMap<String, (String, String)> {
    let mut cache = HashMap::new();
    if let Ok(disk_stats) = procfs::diskstats() {
        for stat in disk_stats {
            if !stat.name.starts_with("loop") && !cache.contains_key(&stat.name) {
                let model = read_disk_model(&stat.name);
                let raid_level = read_raid_level(&stat.name);
                cache.insert(stat.name, (model, raid_level));
            }
        }
    }
    cache
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
            disk_info: Opts::new(LINUX_DISK_INFO, "Disk device info")
                .namespace(NAMESPACE)
                .subsystem(LINUX_DISK_SUBSYSTEM)
                .variable_labels(vec![
                    NAME_LABEL.into(),
                    MODEL_LABEL.into(),
                    RAID_LEVEL_LABEL.into(),
                ])
                .describe()
                .unwrap(),
            disk_info_cache: build_disk_info_cache(),
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
            &self.disk_info,
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

        let mut seen_devices = HashSet::new();
        for mount in mounts {
            if let Some(disk_stat) = disk_stats.get(&mount.majmin) {
                let labels = std::slice::from_ref(&disk_stat.name);
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

                // Emit disk_info only once per device to avoid duplicates from bind mounts
                if seen_devices.insert(disk_stat.name.clone()) {
                    if let Some((model, raid_level)) = self.disk_info_cache.get(&disk_stat.name) {
                        let info_labels: Vec<String> =
                            vec![disk_stat.name.clone(), model.clone(), raid_level.clone()];
                        mfs.extend(
                            ConstMetric::new_gauge(self.disk_info.clone(), 1.0, Some(&info_labels))
                                .unwrap()
                                .collect(),
                        );
                    }
                }
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
