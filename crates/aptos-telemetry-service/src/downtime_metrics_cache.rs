// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    context::Context, prometheus_push_metrics::push_metrics_to_clients, types::auth::Claims,
};
use aptos_infallible::RwLock;
use std::{collections::VecDeque, sync::Arc};
use tokio::time;
use warp::hyper::body::Bytes;

pub struct MetricsEntry {
    pub context: Context,
    pub claims: Claims,
    pub encoding: Option<String>,
    pub metrics_body: Bytes,
    pub timestamp: u64, // The timestamp at which the metrics was originally submitted to a client.
    pub ignore_clients: Vec<String>, // Don't resubmit these metrics to these clients.
}

/// This struct caches the metrics when a client is down.
/// The cache will resubmit the metrics to the client at a later point in time.
pub struct DowntimeMetricsCacheUpdater {
    downtime_metrics: Arc<RwLock<VecDeque<MetricsEntry>>>,
    resubmit_interval: time::Duration,
}

impl DowntimeMetricsCacheUpdater {
    pub fn new(
        downtime_metrics: Arc<RwLock<VecDeque<MetricsEntry>>>,
        resubmit_interval: time::Duration,
    ) -> Self {
        Self {
            downtime_metrics,
            resubmit_interval,
        }
    }

    pub fn run(mut self) {
        let mut interval = time::interval(self.resubmit_interval);
        tokio::spawn(async move {
            loop {
                self.resubmit().await;
                interval.tick().await;
            }
        });
    }

    async fn resubmit(&mut self) {
        let len = self.downtime_metrics.read().len();

        for _i in 0..len {
            let entry = { self.downtime_metrics.write().pop_front() };
            if let Some(entry) = entry {
                let _result = push_metrics_to_clients(
                    entry.context,
                    entry.claims,
                    entry.encoding,
                    entry.metrics_body,
                    entry.timestamp,
                    entry.ignore_clients,
                )
                .await;
            }
        }
    }
}
