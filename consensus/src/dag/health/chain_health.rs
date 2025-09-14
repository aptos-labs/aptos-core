// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters::CHAIN_HEALTH_BACKOFF_TRIGGERED,
    dag::anchor_election::CommitHistory,
    liveness::{leader_reputation::VotingPowerRatio, proposal_generator::ChainHealthBackoffConfig},
};
use aptos_config::config::ChainHealthBackoffValues;
use aptos_consensus_types::common::Round;
use std::{sync::Arc, time::Duration};

pub trait TChainHealth: Send + Sync {
    fn get_round_backoff(&self, round: Round) -> Option<Duration>;

    fn get_round_payload_limits(&self, round: Round) -> Option<(u64, u64)>;

    fn voting_power_ratio(&self, round: Round) -> VotingPowerRatio;
}

pub struct NoChainHealth {}

impl NoChainHealth {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}

impl TChainHealth for NoChainHealth {
    fn get_round_backoff(&self, _round: Round) -> Option<Duration> {
        None
    }

    fn get_round_payload_limits(&self, _round: Round) -> Option<(u64, u64)> {
        None
    }

    fn voting_power_ratio(&self, _round: Round) -> VotingPowerRatio {
        1.0
    }
}

pub struct ChainHealthBackoff {
    config: ChainHealthBackoffConfig,
    commit_history: Arc<dyn CommitHistory>,
}

impl ChainHealthBackoff {
    pub fn new(
        config: ChainHealthBackoffConfig,
        commit_history: Arc<dyn CommitHistory>,
    ) -> Arc<Self> {
        Arc::new(Self {
            commit_history,
            config,
        })
    }

    fn get_chain_health_backoff(&self, round: Round) -> Option<&ChainHealthBackoffValues> {
        let voting_power_ratio = self
            .commit_history
            .get_voting_power_participation_ratio(round);
        self.config.get_backoff(voting_power_ratio)
    }
}

impl TChainHealth for ChainHealthBackoff {
    fn get_round_backoff(&self, round: Round) -> Option<Duration> {
        let chain_health_backoff = self.get_chain_health_backoff(round);

        if let Some(value) = chain_health_backoff {
            CHAIN_HEALTH_BACKOFF_TRIGGERED.observe(1.0);
            Some(Duration::from_millis(value.backoff_proposal_delay_ms))
        } else {
            CHAIN_HEALTH_BACKOFF_TRIGGERED.observe(0.0);
            None
        }
    }

    fn get_round_payload_limits(&self, round: Round) -> Option<(u64, u64)> {
        let chain_health_backoff = self.get_chain_health_backoff(round);

        chain_health_backoff.map(|value| {
            (
                value.max_sending_block_txns_after_filtering_override,
                value.max_sending_block_bytes_override,
            )
        })
    }

    fn voting_power_ratio(&self, round: Round) -> VotingPowerRatio {
        self.commit_history
            .get_voting_power_participation_ratio(round)
    }
}
