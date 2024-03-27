// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::pending_votes::VoteReceptionResult;
use aptos_consensus_types::{
    common::Author,
    order_vote::OrderVote,
};
use aptos_types::validator_verifier::ValidatorVerifier;
use std::collections::HashMap;

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
    // TODO: Finish this
    pub fn insert_order_vote(
        &mut self,
        order_vote: OrderVote,
        validator_verifier: &ValidatorVerifier,
    ) -> VoteReceptionResult {
        let author = order_vote.author();
        let round = order_vote.round();
        //
        // 1. Has the author already voted for this round?
        //
        if let Some(previously_seen_vote) = self.author_to_order_vote.get(&author)
        {
            // we have seen a different vote for the same round
            // error!(
            //     SecurityEvent::ConsensusEquivocatingVote,
            //     remote_peer = order_vote.author(),
            //     vote = order_vote,
            //     previous_vote = previously_seen_vote
            // );
            return VoteReceptionResult::EquivocateVote;
        }

        //
        // 2. Store new vote (or update, in case it's a new timeout vote)
        //
        self.author_to_order_vote
            .insert(order_vote.author(), order_vote.clone());
        VoteReceptionResult::VoteAdded(0)

    }

    pub fn drain_votes(&mut self) -> Vec<OrderVote> {
        self.author_to_order_vote.drain().values().collect()
    }
}
