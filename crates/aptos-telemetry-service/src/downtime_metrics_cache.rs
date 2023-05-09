

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    context::Context,
    types::{auth::Claims},
    prometheus_push_metrics::push_metrics_to_clients
};
use warp::{hyper::body::Bytes};
use std::{
    collections::VecDeque,
    // sync::Arc
};
use tokio::time;

pub struct MetricsEntry {
    pub context: Context,
    pub claims: Claims,
    pub encoding: Option<String>,
    pub metrics_body: Bytes,
    pub timestamp: usize            // The timestamp at which the metrics was originally submitted to a client.
}


impl MetricsEntry {
    pub fn new(
        context: Context,
        claims: Claims,
        encoding: Option<String>,
        metrics_body: Bytes,
        timestamp: usize
    ) -> Self {
        Self {
            context,
            claims,
            encoding,
            metrics_body,
            timestamp
        }
    }
}

/// This struct caches the metrics when a client is down.
/// The cache will resubmit the metrics to the client at a later point in time.
pub struct DowntimeMetricsCache {
    downtime_metrics : VecDeque<MetricsEntry>,
    resubmit_interval: time::Duration,
}

impl DowntimeMetricsCache {
    pub fn new(
        resubmit_interval: time::Duration
    ) -> Self {
        Self {
            downtime_metrics: VecDeque::new(),
            resubmit_interval
        }
    }

    pub fn run(&mut self) {
        let mut interval = time::interval(self.resubmit_interval);
        tokio::spawn(async move {
            loop {
                self.resubmit().await;
                interval.tick().await;
            }
        });
    }

    async fn resubmit(&mut self) {
        let len = self.downtime_metrics.len();

        for _i in 0..len {
            let entry = self.downtime_metrics.pop_front();
            if let Some(entry) = entry {
                push_metrics_to_clients(entry.context, entry.claims, entry.encoding, entry.metrics_body, entry.timestamp).await;
            }
        }
    }

    pub fn add_metrics_to_cache(&mut self,
        context: Context,
        claims: Claims,
        encoding: Option<String>,
        metrics_body: Bytes,
        timestamp: usize
    ) {
        self.downtime_metrics.push_back(MetricsEntry {
            context,
            claims,
            encoding,
            metrics_body,
            timestamp
        })
    }
}
