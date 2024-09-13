// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::proposer_election::ProposerElection;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Round},
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::{error, warn, SecurityEvent};
use std::{cmp::Ordering, sync::Arc};

// Wrapper around ProposerElection.
//
// Provides is_valid_proposal that remembers, and rejects if
// the same leader proposes multiple blocks.
pub struct UnequivocalProposerElection {
    proposer_election: Arc<dyn ProposerElection + Send + Sync>,
    already_proposed_regular: Mutex<(Round, HashValue)>,
    already_proposed_optimistic: Mutex<(Round, HashValue)>,
}

impl ProposerElection for UnequivocalProposerElection {
    fn get_valid_proposer(&self, round: Round) -> Author {
        self.proposer_election.get_valid_proposer(round)
    }

    fn get_voting_power_participation_ratio(&self, round: Round) -> f64 {
        self.proposer_election
            .get_voting_power_participation_ratio(round)
    }
}

impl UnequivocalProposerElection {
    pub fn new(proposer_election: Arc<dyn ProposerElection + Send + Sync>) -> Self {
        Self {
            proposer_election,
            already_proposed_regular: Mutex::new((0, HashValue::zero())),
            already_proposed_optimistic: Mutex::new((0, HashValue::zero())),
        }
    }

    // Return if a given proposed block is valid:
    // - if a given author is a valid candidate for being a proposer
    // - if this is the first block proposer has submitted in this round
    // - if it is not old proposal
    pub fn is_valid_proposal(&self, block: &Block) -> bool {
        block.author().map_or(false, |author| {
            let valid_author = self.is_valid_proposer(author, block.round());
            if !valid_author {
                warn!(
                    SecurityEvent::InvalidConsensusProposal,
                    "Proposal is not from valid author {}, expected {} for round {} and id {}",
                    author,
                    self.get_valid_proposer(block.round()),
                    block.round(),
                    block.id()
                );

                return false;
            }
            if block.is_optimistic_proposal() {
                let mut already_proposed_optimistic = self.already_proposed_optimistic.lock();
                // detect if the leader proposes more than once in this round
                match block.round().cmp(&already_proposed_optimistic.0) {
                    Ordering::Greater => {
                        already_proposed_optimistic.0 = block.round();
                        already_proposed_optimistic.1 = block.id();
                        true
                    },
                    Ordering::Equal => {
                        if already_proposed_optimistic.1 != block.id() {
                            error!(
                                SecurityEvent::InvalidConsensusProposal,
                                "Multiple optimistic proposals from {} for round {}: {} and {}",
                                author,
                                block.round(),
                                already_proposed_optimistic.1,
                                block.id()
                            );
                            false
                        } else {
                            true
                        }
                    },
                    Ordering::Less => false,
                }
            } else {
                let mut already_proposed_regular = self.already_proposed_regular.lock();
                // detect if the leader proposes more than once in this round
                match block.round().cmp(&already_proposed_regular.0) {
                    Ordering::Greater => {
                        already_proposed_regular.0 = block.round();
                        already_proposed_regular.1 = block.id();
                        true
                    },
                    Ordering::Equal => {
                        if already_proposed_regular.1 != block.id() {
                            error!(
                                SecurityEvent::InvalidConsensusProposal,
                                "Multiple regular proposals from {} for round {}: {} and {}",
                                author,
                                block.round(),
                                already_proposed_regular.1,
                                block.id()
                            );
                            false
                        } else {
                            true
                        }
                    },
                    Ordering::Less => false,
                }
            }
        })
    }
}
