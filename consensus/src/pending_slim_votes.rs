// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Holds slim votes and slim order votes that arrived before this node could process them.
//!
//! A slim vote has no `SyncInfo`, so a node that has not entered the vote's round cannot
//! catch up from the vote. A slim order vote has no QC, so a node that has not formed the
//! QC cannot use the order vote yet. This buffer keeps such messages until the node gets
//! the missing certificate from its own vote aggregation, a proposal, or a timeout message.
//!
//! The buffer keeps at most one message per author and round, and only for a bounded
//! window of rounds, so a Byzantine validator cannot grow it without limit.

use aptos_consensus_types::{
    common::{Author, Round},
    order_vote::OrderVote,
    vote::Vote,
};
use aptos_crypto::HashValue;
use std::collections::BTreeMap;

/// The number of rounds above the current round for which votes are kept.
pub const MAX_FUTURE_VOTE_ROUNDS: Round = 2;

/// The number of rounds above the highest ordered round for which order votes are kept.
/// This is the same window that `RoundManager` accepts for regular order votes.
pub const MAX_FUTURE_ORDER_VOTE_ROUNDS: Round = 100;

#[derive(Default)]
pub struct PendingSlimVotes {
    votes: BTreeMap<Round, BTreeMap<Author, Vote>>,
    order_votes: BTreeMap<Round, BTreeMap<Author, OrderVote>>,
}

impl PendingSlimVotes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.votes.is_empty() && self.order_votes.is_empty()
    }

    /// Keeps a vote for a round above `current_round`. Returns false if the vote is outside
    /// the window, or if the buffer already has a vote from the same author for that round.
    pub fn insert_vote(&mut self, current_round: Round, vote: Vote) -> bool {
        let round = vote.vote_data().proposed().round();
        if round <= current_round || round > current_round.saturating_add(MAX_FUTURE_VOTE_ROUNDS) {
            return false;
        }
        let votes = self.votes.entry(round).or_default();
        if votes.contains_key(&vote.author()) {
            return false;
        }
        votes.insert(vote.author(), vote);
        true
    }

    /// Removes the votes for `current_round` and below, and returns those for `current_round`.
    pub fn take_votes(&mut self, current_round: Round) -> Vec<Vote> {
        let later_rounds = self.votes.split_off(&current_round.saturating_add(1));
        std::mem::replace(&mut self.votes, later_rounds)
            .remove(&current_round)
            .map(|votes| votes.into_values().collect())
            .unwrap_or_default()
    }

    /// Keeps an order vote for a round above `highest_ordered_round`. Returns false if the
    /// order vote is outside the window, or if the buffer already has an order vote from the
    /// same author for that round.
    pub fn insert_order_vote(
        &mut self,
        highest_ordered_round: Round,
        order_vote: OrderVote,
    ) -> bool {
        let round = order_vote.ledger_info().round();
        if round <= highest_ordered_round
            || round >= highest_ordered_round.saturating_add(MAX_FUTURE_ORDER_VOTE_ROUNDS)
        {
            return false;
        }
        let order_votes = self.order_votes.entry(round).or_default();
        if order_votes.contains_key(&order_vote.author()) {
            return false;
        }
        order_votes.insert(order_vote.author(), order_vote);
        true
    }

    /// Removes the order votes for `highest_ordered_round` and below. Then removes and returns
    /// the order votes whose block has a QC, as `has_qc` tells.
    pub fn take_ready_order_votes(
        &mut self,
        highest_ordered_round: Round,
        has_qc: impl Fn(HashValue) -> bool,
    ) -> Vec<OrderVote> {
        self.order_votes = self
            .order_votes
            .split_off(&highest_ordered_round.saturating_add(1));
        let ready_rounds: Vec<Round> =
            self.order_votes
                .iter()
                .filter(|(_, order_votes)| {
                    order_votes.values().next().is_some_and(|order_vote| {
                        has_qc(order_vote.ledger_info().consensus_block_id())
                    })
                })
                .map(|(round, _)| *round)
                .collect();
        ready_rounds
            .into_iter()
            .filter_map(|round| self.order_votes.remove(&round))
            .flat_map(|order_votes| order_votes.into_values())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_consensus_types::vote_data::VoteData;
    use aptos_types::{
        block_info::BlockInfo, ledger_info::LedgerInfo, validator_signer::ValidatorSigner,
    };

    fn block_info(round: Round) -> BlockInfo {
        BlockInfo::new(
            1,
            round,
            HashValue::sha3_256_of(&round.to_le_bytes()),
            HashValue::zero(),
            0,
            0,
            None,
        )
    }

    fn vote(signer: &ValidatorSigner, round: Round) -> Vote {
        let vote_data = VoteData::new(block_info(round), block_info(round - 1));
        let ledger_info = LedgerInfo::new(BlockInfo::empty(), vote_data.hash());
        Vote::new(vote_data, signer.author(), ledger_info, signer).unwrap()
    }

    fn order_vote(signer: &ValidatorSigner, round: Round) -> OrderVote {
        let ledger_info = LedgerInfo::new(block_info(round), HashValue::zero());
        OrderVote::new_with_signature(
            signer.author(),
            ledger_info.clone(),
            signer.sign(&ledger_info).unwrap(),
        )
    }

    use aptos_crypto::hash::CryptoHash;

    #[test]
    fn votes_are_bounded_and_released_by_round() {
        let signers: Vec<_> = (0..3).map(|i| ValidatorSigner::random([i; 32])).collect();
        let mut pending = PendingSlimVotes::new();

        // Only rounds in (current, current + MAX_FUTURE_VOTE_ROUNDS] are kept.
        assert!(!pending.insert_vote(10, vote(&signers[0], 10)));
        assert!(pending.insert_vote(10, vote(&signers[0], 11)));
        assert!(pending.insert_vote(10, vote(&signers[1], 11)));
        assert!(pending.insert_vote(10, vote(&signers[0], 12)));
        assert!(!pending.insert_vote(10, vote(&signers[0], 13)));
        // One vote per author and round.
        assert!(!pending.insert_vote(10, vote(&signers[0], 11)));

        assert!(pending.take_votes(10).is_empty());
        let ready = pending.take_votes(11);
        assert_eq!(ready.len(), 2);
        assert!(ready.iter().all(|v| v.vote_data().proposed().round() == 11));

        // Moving past a round drops its votes.
        assert!(pending.take_votes(13).is_empty());
        assert!(pending.is_empty());
    }

    #[test]
    fn order_votes_wait_for_their_qc() {
        let signers: Vec<_> = (0..2).map(|i| ValidatorSigner::random([i; 32])).collect();
        let mut pending = PendingSlimVotes::new();

        assert!(!pending.insert_order_vote(5, order_vote(&signers[0], 5)));
        assert!(!pending
            .insert_order_vote(5, order_vote(&signers[0], 5 + MAX_FUTURE_ORDER_VOTE_ROUNDS)));
        assert!(pending.insert_order_vote(5, order_vote(&signers[0], 6)));
        assert!(pending.insert_order_vote(5, order_vote(&signers[1], 6)));
        assert!(pending.insert_order_vote(5, order_vote(&signers[0], 7)));
        assert!(!pending.insert_order_vote(5, order_vote(&signers[0], 7)));

        // Nothing is released while no QC is local.
        assert!(pending.take_ready_order_votes(5, |_| false).is_empty());

        // Only the round whose block has a QC is released.
        let block_7 = block_info(7).id();
        let ready = pending.take_ready_order_votes(5, |id| id == block_7);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].ledger_info().round(), 7);

        // Ordering past round 6 drops its order votes.
        assert!(pending.take_ready_order_votes(6, |_| true).is_empty());
        assert!(pending.is_empty());
    }
}
