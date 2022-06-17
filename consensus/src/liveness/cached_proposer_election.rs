// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;

use aptos_infallible::Mutex;
use consensus_types::common::{Author, Round};

use super::proposer_election::ProposerElection;

// Wrapper around ProposerElection.
//
// Function get_valid_proposer can be expensive, and we want to make sure
// it is computed only once for a given round.
// Additionally, provides is_valid_proposal that remembers, and rejects if
// the same leader proposes multiple blocks.
pub struct CachedProposerElection {
    proposer_election: Box<dyn ProposerElection + Send + Sync>,
    // We use BTreeMap since we want a fixed window of cached elements
    // to look back (and caller knows how big of a window it needs).
    // LRU cache wouldn't work as well, as access order of the elements
    // would define eviction, and could lead to evicting still needed elements.
    recent_elections: Mutex<BTreeMap<Round, Author>>,
    window: usize,
}

impl CachedProposerElection {
    pub fn new(proposer_election: Box<dyn ProposerElection + Send + Sync>, window: usize) -> Self {
        Self {
            proposer_election,
            recent_elections: Mutex::new(BTreeMap::new()),
            window,
        }
    }
}

impl ProposerElection for CachedProposerElection {
    fn get_valid_proposer(&self, round: Round) -> Author {
        let mut recent_elections = self.recent_elections.lock();

        if round > self.window as u64 {
            *recent_elections = recent_elections.split_off(&(round - self.window as u64));
        }

        *recent_elections
            .entry(round)
            .or_insert_with(|| self.proposer_election.get_valid_proposer(round))
    }
}
