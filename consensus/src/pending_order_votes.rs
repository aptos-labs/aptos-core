// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_consensus_types::{common::Author, order_vote::OrderVote};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::prelude::*;
use aptos_types::{
    aggregate_signature::PartialSignatures,
    ledger_info::{LedgerInfoWithPartialSignatures, LedgerInfoWithSignatures},
    validator_verifier::{ValidatorVerifier, VerifyError},
};
use std::{collections::HashMap, sync::Arc};

/// Result of the order vote processing. The failure case (Verification error) is returned
/// as the Error part of the result.
#[derive(Debug, PartialEq, Eq)]
pub enum OrderVoteReceptionResult {
    /// The vote has been added but QC has not been formed yet. Return the amount of voting power
    /// QC currently has.
    VoteAdded(u128),
    /// The very same vote message has been processed in past.
    DuplicateVote,
    /// The very same author has already voted for another proposal in this round (equivocation).
    EquivocateVote,
    /// This block has just been certified after adding the vote.
    NewLedgerInfoWithSignatures(Arc<LedgerInfoWithSignatures>),
    /// There might be some issues adding a vote
    ErrorAddingVote(VerifyError),
    /// The vote is not for one of the last 2 rounds
    UnexpectedRound(u64, u64),
    /// Error happens when aggregating signature
    ErrorAggregatingSignature(VerifyError),
}

/// A PendingVotes structure keep track of votes
pub struct PendingOrderVotes {
    /// Maps LedgerInfo digest to associated signatures (contained in a partial LedgerInfoWithSignatures).
    /// This might keep multiple LedgerInfos for the current round: either due to different proposals (byzantine behavior)
    /// or due to different NIL proposals (clients can have a different view of what block to extend).
    li_digest_to_votes:
        HashMap<HashValue /* LedgerInfo digest */, (usize, LedgerInfoWithPartialSignatures)>,
    /// Map of Author to (vote, li_digest). This is useful to discard multiple votes.
    author_to_order_vote: HashMap<(Author, u64), (OrderVote, HashValue)>,
}

impl PendingOrderVotes {
    /// Creates an empty PendingOrderVotes structure
    pub fn new() -> Self {
        Self {
            li_digest_to_votes: HashMap::new(),
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
        // derive data from vote
        let li_digest = order_vote.ledger_info().hash();
        let round = order_vote.ledger_info().round();

        if let Some((previously_seen_vote, previous_li_digest)) =
            self.author_to_order_vote.get(&(order_vote.author(), round))
        {
            // is it the same vote?
            if &li_digest == previous_li_digest {
                return OrderVoteReceptionResult::DuplicateVote;
            } else {
                // we have seen a different vote for the same round
                error!(
                    SecurityEvent::ConsensusEquivocatingOrderVote,
                    remote_peer = order_vote.author(),
                    order_vote = order_vote,
                    previous_vote = previously_seen_vote
                );

                return OrderVoteReceptionResult::EquivocateVote;
            }
        }

        self.author_to_order_vote.insert(
            (order_vote.author(), round),
            (order_vote.clone(), li_digest),
        );

        let len = self.li_digest_to_votes.len() + 1;
        // obtain the ledger info with signatures associated to the order vote's ledger info
        let (_hash_index, li_with_sig) =
            self.li_digest_to_votes.entry(li_digest).or_insert_with(|| {
                // if the ledger info with signatures doesn't exist yet, create it
                (
                    len,
                    LedgerInfoWithPartialSignatures::new(
                        order_vote.ledger_info().clone(),
                        PartialSignatures::empty(),
                    ),
                )
            });

        let validator_voting_power = validator_verifier
            .get_voting_power(&order_vote.author())
            .unwrap_or(0);
        if validator_voting_power == 0 {
            warn!(
                "Received vote with no voting power, from {}",
                order_vote.author()
            );
        }
        li_with_sig.add_signature(order_vote.author(), order_vote.signature().clone());

        // check if we have enough signatures to create a QC
        match validator_verifier.check_voting_power(li_with_sig.signatures().keys(), true) {
            // a quorum of signature was reached, a new QC is formed
            Ok(aggregated_voting_power) => {
                assert!(
                    aggregated_voting_power >= validator_verifier.quorum_voting_power(),
                    "QC aggregation should not be triggered if we don't have enough votes to form a QC"
                );
                match li_with_sig.aggregate_signatures(validator_verifier) {
                    Ok(ledger_info_with_sig) => {
                        OrderVoteReceptionResult::NewLedgerInfoWithSignatures(Arc::new(
                            ledger_info_with_sig,
                        ))
                    },
                    Err(e) => OrderVoteReceptionResult::ErrorAggregatingSignature(e),
                }
            },

            // not enough votes
            Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => {
                OrderVoteReceptionResult::VoteAdded(voting_power)
            },

            // error
            Err(error) => {
                error!(
                    "MUST_FIX: order vote received could not be added: {}, order vote: {}",
                    error, order_vote
                );
                OrderVoteReceptionResult::ErrorAddingVote(error)
            },
        }
    }

    // Removes votes older than round-1
    pub fn set_round(&mut self, round: u64) {
        self.li_digest_to_votes
            .retain(|_, (_, li)| li.ledger_info().round() >= round - 1);
        self.author_to_order_vote
            .retain(|&(_, r), _| r >= round - 1);
    }
}
