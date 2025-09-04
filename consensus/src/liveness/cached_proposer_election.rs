// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::proposer_election::ProposerElection;
use crate::counters::PROPOSER_ELECTION_DURATION;
use velor_consensus_types::common::{Author, Round};
use velor_infallible::Mutex;
use velor_logger::prelude::info;
use std::collections::BTreeMap;

// Wrapper around ProposerElection.
//
// Function get_valid_proposer can be expensive, and we want to make sure
// it is computed only once for a given round.
pub struct CachedProposerElection {
    epoch: u64,
    proposer_election: Box<dyn ProposerElection + Send + Sync>,
    // We use BTreeMap since we want a fixed window of cached elements
    // to look back (and caller knows how big of a window it needs).
    // LRU cache wouldn't work as well, as access order of the elements
    // would define eviction, and could lead to evicting still needed elements.
    recent_elections: Mutex<BTreeMap<Round, (Author, f64)>>,
    window: usize,
}

impl CachedProposerElection {
    pub fn new(
        epoch: u64,
        proposer_election: Box<dyn ProposerElection + Send + Sync>,
        window: usize,
    ) -> Self {
        Self {
            epoch,
            proposer_election,
            recent_elections: Mutex::new(BTreeMap::new()),
            window,
        }
    }

    pub fn get_or_compute_entry(&self, round: Round) -> (Author, f64) {
        let mut recent_elections = self.recent_elections.lock();

        if round > self.window as u64 {
            *recent_elections = recent_elections.split_off(&(round - self.window as u64));
        }

        *recent_elections.entry(round).or_insert_with(|| {
            let _timer = PROPOSER_ELECTION_DURATION.start_timer();
            let result = self
                .proposer_election
                .get_valid_proposer_and_voting_power_participation_ratio(round);
            info!(
                "ProposerElection for epoch {} and round {}: {:?}",
                self.epoch, round, result
            );
            result
        })
    }
}

impl ProposerElection for CachedProposerElection {
    fn get_valid_proposer(&self, round: Round) -> Author {
        self.get_or_compute_entry(round).0
    }

    fn get_voting_power_participation_ratio(&self, round: Round) -> f64 {
        self.get_or_compute_entry(round).1
    }
}
