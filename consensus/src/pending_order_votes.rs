// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    counters, pending_votes::VoteReceptionResult, qc_aggregator::{create_qc_aggregator, QcAggregator},
};
use aptos_consensus_types::{
    common::Author,
    delayed_qc_msg::DelayedQcMsg,
    quorum_cert::QuorumCert,
    order_vote::OrderVote,
};
use aptos_types::{
    aggregate_signature::PartialSignatures,
    ledger_info::LedgerInfoWithPartialSignatures,
    validator_verifier::{ValidatorVerifier, VerifyError},
};
use std::{
    collections::HashMap,
    fmt,
};

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
            error!(
                SecurityEvent::ConsensusEquivocatingVote,
                remote_peer = order_vote.author(),
                vote = order_vote,
                previous_vote = previously_seen_vote
            );
            return VoteReceptionResult::EquivocateVote;
        }

        //
        // 2. Store new vote (or update, in case it's a new timeout vote)
        //
        self.author_to_order_vote
            .insert(order_vote.author(), order_vote.clone());

    }

    pub fn drain_votes(&mut self) -> Vec<OrderVote> {
        self.author_to_order_vote.drain().values().collect()
    }
}

impl fmt::Display for PendingOrderVotes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PendingOrderVotes: [round: {}]", self.round)
    }
}

