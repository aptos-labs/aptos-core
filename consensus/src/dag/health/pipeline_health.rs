// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::adapter::OrderedNotifierAdapter, liveness::proposal_generator::PipelineBackpressureConfig,
};
use std::{sync::Arc, time::Duration};

pub trait TPipelineHealth: Send + Sync {
    fn get_backoff(&self) -> Option<Duration>;

    fn get_payload_limits(&self) -> Option<(u64, u64)>;

    fn stop_voting(&self) -> bool;
}

pub struct NoPipelineBackpressure {}

impl NoPipelineBackpressure {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}

impl TPipelineHealth for NoPipelineBackpressure {
    fn get_backoff(&self) -> Option<Duration> {
        None
    }

    fn get_payload_limits(&self) -> Option<(u64, u64)> {
        None
    }

    fn stop_voting(&self) -> bool {
        false
    }
}

pub struct PipelineLatencyBasedBackpressure {
    voter_pipeline_latency_limit: Duration,
    pipeline_config: PipelineBackpressureConfig,
    adapter: Arc<OrderedNotifierAdapter>,
}

impl PipelineLatencyBasedBackpressure {
    pub(in crate::dag) fn new(
        voter_pipeline_latency_limit: Duration,
        pipeline_config: PipelineBackpressureConfig,
        adapter: Arc<OrderedNotifierAdapter>,
    ) -> Arc<Self> {
        Arc::new(Self {
            voter_pipeline_latency_limit,
            pipeline_config,
            adapter,
        })
    }
}

impl TPipelineHealth for PipelineLatencyBasedBackpressure {
    fn get_backoff(&self) -> Option<Duration> {
        let latency = self.adapter.pipeline_pending_latency();
        self.pipeline_config
            .get_backoff(latency)
            .map(|config| Duration::from_millis(config.backpressure_proposal_delay_ms))
    }

    fn get_payload_limits(&self) -> Option<(u64, u64)> {
        let latency = self.adapter.pipeline_pending_latency();
        self.pipeline_config.get_backoff(latency).map(|config| {
            (
                config.max_sending_block_txns_after_filtering_override,
                config.max_sending_block_bytes_override,
            )
        })
    }

    fn stop_voting(&self) -> bool {
        let latency = self.adapter.pipeline_pending_latency();
        latency > self.voter_pipeline_latency_limit
    }
}
