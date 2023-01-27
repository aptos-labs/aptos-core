// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::Lazy;
use prometheus::{
    core::{Collector, Desc},
    proto::MetricFamily,
    register_int_gauge_vec, IntGaugeVec,
};

/// Current host name
pub static HOST_NAME: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!("aptos_node_host_name", "A no-op counter value", &[
        "host_name"
    ])
    .unwrap()
});

/// Git hash of the current release.
pub static NODE_RELEASE_GIT_HASH: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!("aptos_node_release_git_hash", "A no-op counter value", &[
        "git_hash"
    ])
    .unwrap()
});

pub(crate) struct BasicNodeInfoCollector<'a> {
    metrics: &'a Lazy<IntGaugeVec>,
}

impl<'a> BasicNodeInfoCollector<'a> {
    pub(crate) fn new(metrics: &'a Lazy<IntGaugeVec>) -> Self {
        Self { metrics }
    }
}

impl Collector for BasicNodeInfoCollector<'_> {
    fn desc(&self) -> Vec<&Desc> {
        self.metrics.desc()
    }

    fn collect(&self) -> Vec<MetricFamily> {
        self.metrics.collect()
    }
}

pub fn register_basic_node_info_collectors() {
    prometheus::register(Box::new(BasicNodeInfoCollector::new(
        &NODE_RELEASE_GIT_HASH,
    )))
    .unwrap();
    prometheus::register(Box::new(BasicNodeInfoCollector::new(&HOST_NAME))).unwrap();
}
