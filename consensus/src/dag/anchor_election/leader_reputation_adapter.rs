// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        anchor_election::{AnchorElection, CommitHistory},
        storage::CommitEvent,
    },
    liveness::{
        leader_reputation::{
            LeaderReputation, MetadataBackend, ReputationHeuristic, VotingPowerRatio,
        },
        proposer_election::ProposerElection,
    },
    monitor,
};
use aptos_bitvec::BitVec;
use aptos_collections::BoundedVecDeque;
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::info;
use aptos_types::account_config::NewBlockEvent;
use move_core_types::account_address::AccountAddress;
use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
};

pub struct MetadataBackendAdapter {
    epoch_to_validators: HashMap<u64, HashMap<Author, usize>>,
    window_size: usize,
    sliding_window: Mutex<BoundedVecDeque<CommitEvent>>,
}

impl MetadataBackendAdapter {
    pub fn new(
        window_size: usize,
        epoch_to_validators: HashMap<u64, HashMap<Author, usize>>,
    ) -> Self {
        Self {
            epoch_to_validators,
            window_size,
            sliding_window: Mutex::new(BoundedVecDeque::new(window_size)),
        }
    }

    pub fn push(&self, event: CommitEvent) {
        if !self.epoch_to_validators.contains_key(&event.epoch()) {
            return;
        }
        self.sliding_window.lock().push_front(event);
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
}

impl LeaderReputationAdapter {
    pub fn new(
        epoch: u64,
        epoch_to_proposers: HashMap<u64, Vec<Author>>,
        voting_powers: Vec<u64>,
        backend: Arc<MetadataBackendAdapter>,
        heuristic: Box<dyn ReputationHeuristic>,
        window_for_chain_health: usize,
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
        }
    }
}

impl AnchorElection for LeaderReputationAdapter {
    fn get_anchor(&self, round: Round) -> Author {
        monitor!("dag_get_anchor", self.reputation.get_valid_proposer(round))
    }

    fn update_reputation(&self, commit_event: CommitEvent) {
        monitor!("dag_update_reputation", self.data_source.push(commit_event))
    }
}

impl CommitHistory for LeaderReputationAdapter {
    fn get_voting_power_participation_ratio(&self, round: Round) -> VotingPowerRatio {
        let mut voting_power_ratio = monitor!(
            "dag_get_voting_power_ratio",
            self.reputation.get_voting_power_participation_ratio(round)
        );
        // TODO: fix this once leader reputation is fixed
        if voting_power_ratio < 0.67 {
            voting_power_ratio = 1.0;
        }

        voting_power_ratio
    }
}

pub struct CachedLeaderReputation {
    epoch: u64,
    inner: LeaderReputationAdapter,
    recent_elections: Mutex<BTreeMap<Round, (Author, f64)>>,
}

impl CachedLeaderReputation {
    pub fn new(epoch: u64, leader_reputation: LeaderReputationAdapter) -> Self {
        Self {
            epoch,
            inner: leader_reputation,
            recent_elections: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn get_or_compute_entry(&self, round: Round) -> (Author, f64) {
        let mut recent_elections = self.recent_elections.lock();

        *recent_elections.entry(round).or_insert_with(|| {
            let result = monitor!(
                "dag_compute_leader_rep",
                self.inner
                    .reputation
                    .get_valid_proposer_and_voting_power_participation_ratio(round)
            );
            info!(
                "AnchorElection for epoch {} and round {}: {:?}",
                self.epoch, round, result
            );
            result
        })
    }
}

impl AnchorElection for CachedLeaderReputation {
    fn get_anchor(&self, round: Round) -> Author {
        monitor!("dag_get_anchor", self.get_or_compute_entry(round).0)
    }

    fn update_reputation(&self, commit_event: CommitEvent) {
        self.inner.update_reputation(commit_event);
        self.recent_elections.lock().clear();
    }
}

impl CommitHistory for CachedLeaderReputation {
    fn get_voting_power_participation_ratio(&self, round: Round) -> VotingPowerRatio {
        self.get_or_compute_entry(round).1
    }
}
