// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::cmp::Ordering;

use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_logger::{error, SecurityEvent};
use consensus_types::{
    block::Block,
    common::{Author, Round},
};

use super::proposer_election::ProposerElection;

// Wrapper around ProposerElection.
//
// Provides is_valid_proposal that remembers, and rejects if
// the same leader proposes multiple blocks.
pub struct UnequivocalProposerElection {
    proposer_election: Box<dyn ProposerElection + Send + Sync>,
    already_proposed: Mutex<(Round, HashValue)>,
}

impl ProposerElection for UnequivocalProposerElection {
    fn get_valid_proposer(&self, round: Round) -> Author {
        self.proposer_election.get_valid_proposer(round)
    }
}

impl UnequivocalProposerElection {
    pub fn new(proposer_election: Box<dyn ProposerElection + Send + Sync>) -> Self {
        Self {
            proposer_election,
            already_proposed: Mutex::new((0, HashValue::zero())),
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
                error!(
                    SecurityEvent::InvalidConsensusProposal,
                    "Proposal is not from valid author {}, expected {} for round {} and id {}",
                    author,
                    self.get_valid_proposer(block.round()),
                    block.round(),
                    block.id()
                );

                println!("Not a valid author");
                return false;
            }
            let mut already_proposed = self.already_proposed.lock();
            // detect if the leader proposes more than once in this round
            match block.round().cmp(&already_proposed.0) {
                Ordering::Greater => {
                    already_proposed.0 = block.round();
                    already_proposed.1 = block.id();
                    true
                }
                Ordering::Equal => {
                    if already_proposed.1 != block.id() {
                        error!(
                            SecurityEvent::InvalidConsensusProposal,
                            "Multiple proposals from {} for round {}: {} and {}",
                            author,
                            block.round(),
                            already_proposed.1,
                            block.id()
                        );
                        false
                    } else {
                        true
                    }
                }
                Ordering::Less => {
                    println!("Older Block");
                    false
                }
            }
        })
    }
}
