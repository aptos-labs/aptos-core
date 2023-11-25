// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::TChainHealthBackoff;
use crate::{
    counters::CHAIN_HEALTH_BACKOFF_TRIGGERED,
    dag::{anchor_election::AnchorElection, storage::CommitEvent},
    liveness::{
        leader_reputation::{LeaderReputation, MetadataBackend, ReputationHeuristic},
        proposal_generator::ChainHealthBackoffConfig,
        proposer_election::ProposerElection,
    },
};
use aptos_bitvec::BitVec;
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_types::account_config::NewBlockEvent;
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::Duration,
};

pub struct MetadataBackendAdapter {
    epoch_to_validators: HashMap<u64, HashMap<Author, usize>>,
    window_size: usize,
    sliding_window: Mutex<VecDeque<CommitEvent>>,
}

impl MetadataBackendAdapter {
    pub fn new(
        window_size: usize,
        epoch_to_validators: HashMap<u64, HashMap<Author, usize>>,
    ) -> Self {
        Self {
            epoch_to_validators,
            window_size,
            sliding_window: Mutex::new(VecDeque::new()),
        }
    }

    pub fn push(&self, event: CommitEvent) {
        if !self.epoch_to_validators.contains_key(&event.epoch()) {
            return;
        }
        let mut lock = self.sliding_window.lock();
        if lock.len() == self.window_size {
            lock.pop_back();
        }
        lock.push_front(event);
    }

    // TODO: we should change NewBlockEvent on LeaderReputation to take a trait
    fn convert(&self, event: CommitEvent) -> NewBlockEvent {
        let validators = self.epoch_to_validators.get(&event.epoch()).unwrap();
        let mut bitvec = BitVec::with_num_bits(validators.len() as u16);
        for author in event.parents() {
            bitvec.set(*validators.get(author).unwrap() as u16);
        }
        let mut failed_authors = vec![];
        for author in event.failed_authors() {
            failed_authors.push(*validators.get(author).unwrap() as u64);
        }
        NewBlockEvent::new(
            AccountAddress::ZERO,
            event.epoch(),
            event.round(),
            0,
            bitvec.into(),
            *event.author(),
            failed_authors,
            0,
        )
    }
}

impl MetadataBackend for MetadataBackendAdapter {
    fn get_block_metadata(
        &self,
        _target_epoch: u64,
        _target_round: Round,
    ) -> (Vec<NewBlockEvent>, HashValue) {
        let events: Vec<_> = self
            .sliding_window
            .lock()
            .clone()
            .into_iter()
            .map(|event| self.convert(event))
            .collect();
        (
            events,
            // TODO: fill in the hash value
            HashValue::zero(),
        )
    }
}

pub struct LeaderReputationAdapter {
    reputation: LeaderReputation,
    data_source: Arc<MetadataBackendAdapter>,
    chain_health_backoff_config: ChainHealthBackoffConfig,
}

impl LeaderReputationAdapter {
    pub fn new(
        epoch: u64,
        epoch_to_proposers: HashMap<u64, Vec<Author>>,
        voting_powers: Vec<u64>,
        backend: Arc<MetadataBackendAdapter>,
        heuristic: Box<dyn ReputationHeuristic>,
        window_for_chain_health: usize,
        chain_health_backoff_config: ChainHealthBackoffConfig,
    ) -> Self {
        Self {
            reputation: LeaderReputation::new(
                epoch,
                epoch_to_proposers,
                voting_powers,
                backend.clone(),
                heuristic,
                0,
                true,
                window_for_chain_health,
            ),
            data_source: backend,
            chain_health_backoff_config,
        }
    }

    fn get_chain_health_backoff(
        &self,
        round: u64,
    ) -> (f64, Option<&aptos_config::config::ChainHealthBackoffValues>) {
        let mut voting_power_ratio = self.reputation.get_voting_power_participation_ratio(round);
        // TODO: fix this once leader reputation is fixed
        if voting_power_ratio < 0.67 {
            voting_power_ratio = 1.0;
        }

        let chain_health_backoff = self
            .chain_health_backoff_config
            .get_backoff(voting_power_ratio);
        (voting_power_ratio, chain_health_backoff)
    }
}

impl AnchorElection for LeaderReputationAdapter {
    fn get_anchor(&self, round: Round) -> Author {
        self.reputation.get_valid_proposer(round)
    }

    fn update_reputation(&self, commit_event: CommitEvent) {
        self.data_source.push(commit_event)
    }
}

impl TChainHealthBackoff for LeaderReputationAdapter {
    fn get_round_backoff(&self, round: Round) -> (f64, Option<Duration>) {
        let (voting_power_ratio, chain_health_backoff) = self.get_chain_health_backoff(round);
        let backoff_duration = if let Some(value) = chain_health_backoff {
            CHAIN_HEALTH_BACKOFF_TRIGGERED.observe(1.0);
            Some(Duration::from_millis(value.backoff_proposal_delay_ms))
        } else {
            CHAIN_HEALTH_BACKOFF_TRIGGERED.observe(0.0);
            None
        };
        (voting_power_ratio, backoff_duration)
    }

    fn get_round_payload_limits(&self, round: Round) -> (f64, Option<(u64, u64)>) {
        let (voting_power_ratio, chain_health_backoff) = self.get_chain_health_backoff(round);
        let backoff_limits = chain_health_backoff.map(|value| {
            (
                value.max_sending_block_txns_override,
                value.max_sending_block_bytes_override,
            )
        });
        (voting_power_ratio, backoff_limits)
    }
}
