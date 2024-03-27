// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::{common::Author, order_vote::OrderVote};
use aptos_logger::prelude::*;
use aptos_types::validator_verifier::{ValidatorVerifier, VerifyError};
use std::collections::HashMap;

/// Result of the order vote processing. The failure case (Verification error) is returned
/// as the Error part of the result.
#[derive(Debug, PartialEq, Eq)]
pub enum OrderVoteReceptionResult {
    /// The vote has been added but QC has not been formed yet. Return the amount of voting power
    /// QC currently has.
    VoteAdded(u128),
    /// The very same author has already voted for another proposal in this round (equivocation).
    EquivocateVote,
    /// This block has just been certified after adding the vote.
    NewQuorumCertificate,
    /// There might be some issues adding a vote
    ErrorAddingVote(VerifyError),
    /// The vote is not for the current round.
    UnexpectedRound(u64, u64),
}

/// A PendingVotes structure keep track of votes
pub struct PendingOrderVotes {
    /// Map of Author to OrderVote.
    author_to_order_vote: HashMap<Author, OrderVote>,
}

impl PendingOrderVotes {
    /// Creates an empty PendingOrderVotes structure for a specific epoch and round
    pub fn new() -> Self {
        Self {
            author_to_order_vote: HashMap::new(),
        }
    }

    /// Add a vote to the pending votes
    // TODO: Should we add any counters here?
    pub fn insert_order_vote(
        &mut self,
        order_vote: &OrderVote,
        validator_verifier: &ValidatorVerifier,
    ) -> OrderVoteReceptionResult {
        let author = order_vote.author();
        // TODO: Need to make sure the order vote is for the previous round.

        if let Some(previously_seen_vote) = self.author_to_order_vote.get(&author) {
            // we have seen a different vote for the same round
            error!(
                SecurityEvent::ConsensusEquivocatingOrderVote,
                remote_peer = order_vote.author(),
                vote = order_vote,
                previous_vote = previously_seen_vote
            );
            return OrderVoteReceptionResult::EquivocateVote;
        }

        self.author_to_order_vote
            .insert(order_vote.author(), order_vote.clone());

        let validator_voting_power = validator_verifier
            .get_voting_power(&order_vote.author())
            .unwrap_or(0);
        if validator_voting_power == 0 {
            warn!(
                "Received vote with no voting power, from {}",
                order_vote.author()
            );
        }

        // check if we have enough signatures to create a QC
        let voting_power =
            match validator_verifier.check_voting_power(self.author_to_order_vote.keys(), true) {
                // a quorum of signature was reached, a new QC is formed
                Ok(_aggregated_voting_power) => {
                    return OrderVoteReceptionResult::NewQuorumCertificate;
                },

                // not enough votes
                Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => voting_power,

                // error
                Err(error) => {
                    error!(
                        "MUST_FIX: order vote received could not be added: {}, order vote: {}",
                        error, order_vote
                    );
                    return OrderVoteReceptionResult::ErrorAddingVote(error);
                },
            };

        OrderVoteReceptionResult::VoteAdded(voting_power)
    }
}
