use crate::{
    dag::adapter::OrderedNotifierAdapter, liveness::proposal_generator::PipelineBackpressureConfig,
};
use std::{sync::Arc, time::Duration};

pub trait TPipelineHealth: Send + Sync {
    fn get_backoff(&self) -> Option<Duration>;

    fn get_payload_limits(&self) -> Option<(u64, u64)>;
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
}

pub struct PipelineLatencyBasedBackpressure {
    pipeline_config: PipelineBackpressureConfig,
    adapter: Arc<OrderedNotifierAdapter>,
}

impl PipelineLatencyBasedBackpressure {
    pub(in crate::dag) fn new(
        pipeline_config: PipelineBackpressureConfig,
        adapter: Arc<OrderedNotifierAdapter>,
    ) -> Arc<Self> {
        Arc::new(Self {
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
                config.max_sending_block_txns_override,
                config.max_sending_block_bytes_override,
            )
        })
    }
}
