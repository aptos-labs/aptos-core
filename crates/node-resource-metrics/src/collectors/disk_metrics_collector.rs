// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
use sysinfo::{DiskExt, RefreshKind, System, SystemExt};

const DISK_SUBSYSTEM: &str = "disk";

const TOTAL_SPACE: &str = "total_space";
const AVAILABLE_SPACE: &str = "available_space";

const NAME_LABEL: &str = "name";
const TYPE_LABEL: &str = "type";
const FILE_SYSTEM_LABEL: &str = "file_system";

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

        system
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
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::DiskMetricsCollector;
    use prometheus::Registry;

    #[test]
    fn test_disk_collector_register() {
        let collector = DiskMetricsCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
