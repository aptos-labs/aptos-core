// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::sync::Arc;

use super::common::NAMESPACE;
use aptos_infallible::Mutex;
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use sysinfo::{DiskExt, RefreshKind, System, SystemExt};

const DISK_SUBSYSTEM: &str = "disk";

/// A Collector for exposing Disk metrics
pub(crate) struct DiskCollector {
    system: Arc<Mutex<System>>,

    total_space: Desc,
    available_space: Desc,
}

impl DiskCollector {
    fn new() -> Self {
        let system = Arc::new(Mutex::new(System::new_with_specifics(
            RefreshKind::new().with_disks_list().with_disks(),
        )));
        let total_space = Opts::new("total_space", "Total disk size in bytes")
            .namespace(NAMESPACE)
            .subsystem(DISK_SUBSYSTEM)
            .variable_labels(vec!["name".into(), "type".into(), "file_system".into()])
            .describe()
            .unwrap();
        let available_space = Opts::new("available_space", "Total available disk size in bytes")
            .namespace(NAMESPACE)
            .subsystem(DISK_SUBSYSTEM)
            .variable_labels(vec!["name".into(), "type".into(), "file_system".into()])
            .describe()
            .unwrap();

        Self {
            system,
            total_space,
            available_space,
        }
    }
}

impl Default for DiskCollector {
    fn default() -> Self {
        DiskCollector::new()
    }
}

impl Collector for DiskCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![&self.total_space, &self.available_space]
    }

    fn collect(&self) -> Vec<MetricFamily> {
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

#[cfg(test)]
mod tests {
    use super::DiskCollector;
    use prometheus::Registry;

    #[test]
    fn test_disk_collector_register() {
        let collector = DiskCollector::default();

        let r = Registry::new();
        let res = r.register(Box::new(collector));
        assert!(res.is_ok());
    }
}
