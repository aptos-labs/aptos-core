use crate::{
    counters::CHAIN_HEALTH_BACKOFF_TRIGGERED,
    dag::anchor_election::LeaderReputationAdapter,
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
    adapter: Arc<LeaderReputationAdapter>,
}

impl ChainHealthBackoff {
    pub fn new(
        config: ChainHealthBackoffConfig,
        adapter: Arc<LeaderReputationAdapter>,
    ) -> Arc<Self> {
        Arc::new(Self { adapter, config })
    }

    fn get_chain_health_backoff(&self, round: Round) -> Option<&ChainHealthBackoffValues> {
        let voting_power_ratio = self.adapter.get_voting_power_participation_ratio(round);
        let chain_health_backoff = self.config.get_backoff(voting_power_ratio);

        chain_health_backoff
    }
}

impl TChainHealth for ChainHealthBackoff {
    fn get_round_backoff(&self, round: Round) -> Option<Duration> {
        let chain_health_backoff = self.get_chain_health_backoff(round);
        let backoff_duration = if let Some(value) = chain_health_backoff {
            CHAIN_HEALTH_BACKOFF_TRIGGERED.observe(1.0);
            Some(Duration::from_millis(value.backoff_proposal_delay_ms))
        } else {
            CHAIN_HEALTH_BACKOFF_TRIGGERED.observe(0.0);
            None
        };
        backoff_duration
    }

    fn get_round_payload_limits(&self, round: Round) -> Option<(u64, u64)> {
        let chain_health_backoff = self.get_chain_health_backoff(round);
        let backoff_limits = chain_health_backoff.map(|value| {
            (
                value.max_sending_block_txns_override,
                value.max_sending_block_bytes_override,
            )
        });
        backoff_limits
    }

    fn voting_power_ratio(&self, round: Round) -> VotingPowerRatio {
        self.adapter.get_voting_power_participation_ratio(round)
    }
}
