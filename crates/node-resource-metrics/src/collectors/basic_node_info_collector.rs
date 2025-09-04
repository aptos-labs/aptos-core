// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::collectors::common::NAMESPACE;
use velor_infallible::Mutex;
use velor_metrics_core::const_metric::ConstMetric;
use prometheus::{
    core::{Collector, Desc, Describer},
    proto::MetricFamily,
    Opts,
};
use std::{collections::BTreeMap, sync::Arc};
use sysinfo::{RefreshKind, System, SystemExt};

const BASIC_NODE_INFO_METRICS_COUNT: usize = 2;
const RELEASE_GIT_HASH_LABEL: &str = "release_git_hash";
const RELEASE_VERSION_LABEL: &str = "release_version";
const NODE_HOSTNAME_LABEL: &str = "hostname";

const GIT_HASH_LABEL: &str = "git_hash";
const VERSION_LABEL: &str = "version";
const HOSTNAME_LABEL: &str = "name";

const UNKNOW_LABEL: &str = "unknown";

pub(crate) struct BasicNodeInfoCollector {
    release_metric: ConstMetric,
    hostname_metric: ConstMetric,
    version_metric: ConstMetric,
}

impl BasicNodeInfoCollector {
    pub fn new(maybe_build_info: Option<&BTreeMap<String, String>>) -> Self {
        let system = Arc::new(Mutex::new(System::new_with_specifics(RefreshKind::new())));
        let mut fallback_build_info = BTreeMap::new();

        let build_info = if let Some(build_info) = maybe_build_info {
            build_info
        } else {
            let git_hash = velor_build_info::get_git_hash();
            fallback_build_info.insert(velor_build_info::BUILD_COMMIT_HASH.into(), git_hash);
            &fallback_build_info
        };

        let release_hash_desc = Opts::new(RELEASE_GIT_HASH_LABEL, "Release git hash.")
            .namespace(NAMESPACE)
            .variable_label(GIT_HASH_LABEL)
            .describe()
            .unwrap();
        let release_version_desc = Opts::new(RELEASE_VERSION_LABEL, "Release version")
            .namespace(NAMESPACE)
            .variable_label(VERSION_LABEL)
            .describe()
            .unwrap();
        let hostname_desc = Opts::new(NODE_HOSTNAME_LABEL, "Hostname.")
            .namespace(NAMESPACE)
            .variable_label(HOSTNAME_LABEL)
            .describe()
            .unwrap();

        let git_hash = build_info
            .get(velor_build_info::BUILD_COMMIT_HASH)
            .cloned()
            .unwrap_or_else(|| String::from(UNKNOW_LABEL));
        let release_metric =
            ConstMetric::new_gauge(release_hash_desc, 1.0, Some(&[git_hash])).unwrap();

        let node_version = build_info
            .get(velor_build_info::BUILD_PKG_VERSION)
            .cloned()
            .unwrap_or_else(|| String::from(UNKNOW_LABEL));
        let version_metric =
            ConstMetric::new_gauge(release_version_desc, 1.0, Some(&[node_version])).unwrap();

        let hostname = system
            .lock()
            .host_name()
            .unwrap_or_else(|| String::from(UNKNOW_LABEL));

        let hostname_metric =
            ConstMetric::new_gauge(hostname_desc, 1.0, Some(&[hostname])).unwrap();

        Self {
            release_metric,
            version_metric,
            hostname_metric,
        }
    }
}

impl Collector for BasicNodeInfoCollector {
    fn desc(&self) -> Vec<&Desc> {
        self.hostname_metric
            .desc()
            .into_iter()
            .chain(self.release_metric.desc())
            .chain(self.version_metric.desc())
            .collect()
    }

    fn collect(&self) -> Vec<MetricFamily> {
        let mut mfs = Vec::with_capacity(BASIC_NODE_INFO_METRICS_COUNT);
        mfs.extend(self.hostname_metric.collect());
        mfs.extend(self.release_metric.collect());
        mfs.extend(self.version_metric.collect());
        mfs
    }
}
