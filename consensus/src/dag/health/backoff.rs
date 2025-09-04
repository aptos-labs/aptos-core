// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{pipeline_health::TPipelineHealth, TChainHealth};
use velor_config::config::DagPayloadConfig;
use velor_consensus_types::common::Round;
use velor_types::epoch_state::EpochState;
use std::{sync::Arc, time::Duration};

#[derive(Clone)]
pub struct HealthBackoff {
    epoch_state: Arc<EpochState>,
    chain_health: Arc<dyn TChainHealth>,
    pipeline_health: Arc<dyn TPipelineHealth>,
}

impl HealthBackoff {
    pub fn new(
        epoch_state: Arc<EpochState>,
        chain_health: Arc<dyn TChainHealth>,
        pipeline_health: Arc<dyn TPipelineHealth>,
    ) -> Self {
        Self {
            epoch_state,
            chain_health,
            pipeline_health,
        }
    }

    pub fn calculate_payload_limits(
        &self,
        round: Round,
        payload_config: &DagPayloadConfig,
    ) -> (u64, u64) {
        let chain_backoff = self
            .chain_health
            .get_round_payload_limits(round)
            .unwrap_or((u64::MAX, u64::MAX));
        let pipeline_backoff = self
            .pipeline_health
            .get_payload_limits()
            .unwrap_or((u64::MAX, u64::MAX));
        let voting_power_ratio = self.chain_health.voting_power_ratio(round);

        let max_txns_per_round = [
            payload_config.max_sending_txns_per_round,
            chain_backoff.0,
            pipeline_backoff.0,
        ]
        .into_iter()
        .min()
        .expect("must not be empty");

        let max_size_per_round_bytes = [
            payload_config.max_sending_size_per_round_bytes,
            chain_backoff.1,
            pipeline_backoff.1,
        ]
        .into_iter()
        .min()
        .expect("must not be empty");

        // TODO: figure out receiver side checks
        let max_txns = max_txns_per_round.saturating_div(
            (self.epoch_state.verifier.len() as f64 * voting_power_ratio).ceil() as u64,
        );
        let max_txn_size_bytes = max_size_per_round_bytes.saturating_div(
            (self.epoch_state.verifier.len() as f64 * voting_power_ratio).ceil() as u64,
        );

        (max_txns, max_txn_size_bytes)
    }

    pub fn backoff_duration(&self, round: Round) -> Duration {
        let chain_backoff = self.chain_health.get_round_backoff(round);
        let pipeline_backoff = self.pipeline_health.get_backoff();

        chain_backoff
            .unwrap_or_default()
            .max(pipeline_backoff.unwrap_or_default())
    }

    pub fn stop_voting(&self) -> bool {
        self.pipeline_health.stop_voting()
    }
}
