// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::collectors::common::NAMESPACE;
use aptos_infallible::Mutex;
use aptos_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::sync::Arc;
use sysinfo::{RefreshKind, System, SystemExt};

const BASIC_NODE_INFO_METRICS_COUNT: usize = 2;
const RELEASE_GIT_HASH_LABEL: &str = "release_git_hash";
const NODE_HOST_NAME_LABEL: &str = "host_name";

const GIT_HASH_LABEL: &str = "git_hash";
const HOST_NAME_LABEL: &str = "name";

const UNKNOW_LABEL: &str = "unknown";

pub(crate) struct BasicNodeInfoCollector {
    system: Arc<Mutex<System>>,
    release: Desc,
    host_name: Desc,
}

impl BasicNodeInfoCollector {
    fn new() -> Self {
        let system = Arc::new(Mutex::new(System::new_with_specifics(RefreshKind::new())));

        let release = Opts::new(RELEASE_GIT_HASH_LABEL, "Release git hash.")
            .namespace(NAMESPACE)
            .variable_label(GIT_HASH_LABEL)
            .describe()
            .unwrap();
        let host_name = Opts::new(NODE_HOST_NAME_LABEL, "Host name.")
            .namespace(NAMESPACE)
            .variable_label(HOST_NAME_LABEL)
            .describe()
            .unwrap();

        Self {
            system,
            release,
            host_name,
        }
    }
}

impl Default for BasicNodeInfoCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Collector for BasicNodeInfoCollector {
    fn desc(&self) -> Vec<&Desc> {
        vec![&self.host_name, &self.release]
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let host_name = self
            .system
            .lock()
            .host_name()
            .unwrap_or_else(|| String::from(UNKNOW_LABEL));

        let host_name_metrics =
            ConstMetric::new_gauge(self.host_name.clone(), 1.0, Some(&[host_name])).unwrap();

        let git_hash = aptos_build_info::get_git_hash();
        let release_metrics =
            ConstMetric::new_gauge(self.release.clone(), 1.0, Some(&[git_hash])).unwrap();

        let mut mfs = Vec::with_capacity(BASIC_NODE_INFO_METRICS_COUNT);
        mfs.extend(host_name_metrics.collect());
        mfs.extend(release_metrics.collect());
        mfs
    }
}
