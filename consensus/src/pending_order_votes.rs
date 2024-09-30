// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::counters;
use aptos_consensus_types::{common::Author, order_vote::OrderVote, quorum_cert::QuorumCert};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::prelude::*;
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures, LedgerInfoWithUnverifiedSignatures},
    validator_verifier::VerifyError,
};
use std::{collections::HashMap, sync::Arc};

/// Result of the order vote processing. The failure case (Verification error) is returned
/// as the Error part of the result.
#[derive(Debug, PartialEq, Eq)]
pub enum OrderVoteReceptionResult {
    /// The vote has been added but QC has not been formed yet. Return the amount of voting power
    /// QC currently has.
    VoteAdded(u128),
    /// This block has just been certified after adding the vote.
    /// Returns the created order certificate and the QC on which the order certificate is based.
    NewLedgerInfoWithSignatures((Arc<QuorumCert>, LedgerInfoWithSignatures)),
    /// There might be some issues adding a vote
    ErrorAddingVote(VerifyError),
    /// Error happens when aggregating signature
    ErrorAggregatingSignature(VerifyError),
    /// The author of the order vote is unknown
    UnknownAuthor(Author),
}

#[derive(Debug, PartialEq, Eq)]
enum OrderVoteStatus {
    EnoughVotes(LedgerInfoWithSignatures),
    NotEnoughVotes(LedgerInfoWithUnverifiedSignatures),
}

/// A PendingVotes structure keep track of order votes for the last few rounds
pub struct PendingOrderVotes {
    /// Maps LedgerInfo digest to associated signatures.
    /// Order vote status stores caches the information on whether the votes are enough to form a QC.
    /// We also store the QC that the order votes certify.
    li_digest_to_votes:
        HashMap<HashValue /* LedgerInfo digest */, (QuorumCert, OrderVoteStatus)>,
}

impl PendingOrderVotes {
    /// Creates an empty PendingOrderVotes structure
    pub fn new() -> Self {
        Self {
            li_digest_to_votes: HashMap::new(),
        }
    }

    pub fn exists(&self, li_digest: &HashValue) -> bool {
        self.li_digest_to_votes.contains_key(li_digest)
    }

    /// Add a vote to the pending votes
    // TODO: Should we add any counters here?
    pub fn insert_order_vote(
        &mut self,
        order_vote: &OrderVote,
        epoch_state: Arc<EpochState>,
        verified_quorum_cert: Option<QuorumCert>,
    ) -> OrderVoteReceptionResult {
        // derive data from order vote
        let li_digest = order_vote.ledger_info().hash();

        // obtain the ledger info with signatures associated to the order vote's ledger info
        let (quorum_cert, status) = self.li_digest_to_votes.entry(li_digest).or_insert_with(|| {
            // if the ledger info with signatures doesn't exist yet, create it
            (
                verified_quorum_cert.expect(
                    "Quorum Cert is expected when creating a new entry in pending order votes",
                ),
                OrderVoteStatus::NotEnoughVotes(LedgerInfoWithUnverifiedSignatures::new(
                    order_vote.ledger_info().clone(),
                )),
            )
        });

        match status {
            OrderVoteStatus::EnoughVotes(li_with_sig) => {
                // we already have enough votes for this ledger info
                OrderVoteReceptionResult::NewLedgerInfoWithSignatures((
                    Arc::new(quorum_cert.clone()),
                    li_with_sig.clone(),
                ))
            },
            OrderVoteStatus::NotEnoughVotes(li_with_sig) => {
                // we don't have enough votes for this ledger info yet
                let validator_voting_power =
                    epoch_state.verifier.get_voting_power(&order_vote.author());
                if validator_voting_power.is_none() {
                    warn!(
                        "Received order vote from an unknown author: {}",
                        order_vote.author()
                    );
                    return OrderVoteReceptionResult::UnknownAuthor(order_vote.author());
                }
                let validator_voting_power =
                    validator_voting_power.expect("Author must exist in the validator set.");

                if validator_voting_power == 0 {
                    warn!(
                        "Received vote with no voting power, from {}",
                        order_vote.author()
                    );
                }
                li_with_sig.add_signature(order_vote.author(), order_vote.signature_with_status());
                match li_with_sig.check_voting_power(&epoch_state.verifier, true) {
                    Ok(aggregated_voting_power) => {
                        assert!(
                            aggregated_voting_power >= epoch_state.verifier.quorum_voting_power(),
                            "QC aggregation should not be triggered if we don't have enough votes to form a QC"
                        );
                        let verification_result = {
                            let _timer = counters::VERIFY_MSG
                                .with_label_values(&["order_vote_aggregate_and_verify"])
                                .start_timer();
                            li_with_sig.aggregate_and_verify(epoch_state.clone())
                        };
                        match verification_result {
                            Ok(ledger_info_with_sig) => {
                                *status =
                                    OrderVoteStatus::EnoughVotes(ledger_info_with_sig.clone());
                                OrderVoteReceptionResult::NewLedgerInfoWithSignatures((
                                    Arc::new(quorum_cert.clone()),
                                    ledger_info_with_sig,
                                ))
                            },
                            Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => {
                                OrderVoteReceptionResult::VoteAdded(voting_power)
                            },
                            Err(e) => OrderVoteReceptionResult::ErrorAggregatingSignature(e),
                        }
                    },
                    Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => {
                        OrderVoteReceptionResult::VoteAdded(voting_power)
                    },
                    Err(error) => {
                        error!(
                            "MUST_FIX: order vote received could not be added: {}, order vote: {}",
                            error, order_vote
                        );
                        OrderVoteReceptionResult::ErrorAddingVote(error)
                    },
                }
            },
        }
    }

    // Removes votes older than highest_ordered_round
    pub fn garbage_collect(&mut self, highest_ordered_round: u64) {
        self.li_digest_to_votes
            .retain(|_, (_, status)| match status {
                OrderVoteStatus::EnoughVotes(li_with_sig) => {
                    li_with_sig.ledger_info().round() > highest_ordered_round
                },
                OrderVoteStatus::NotEnoughVotes(li_with_sig) => {
                    li_with_sig.ledger_info().round() > highest_ordered_round
                },
            });
    }

    pub fn has_enough_order_votes(&self, ledger_info: &LedgerInfo) -> bool {
        let li_digest = ledger_info.hash();
        if let Some((_, OrderVoteStatus::EnoughVotes(_))) = self.li_digest_to_votes.get(&li_digest)
        {
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::{OrderVoteReceptionResult, PendingOrderVotes};
    use aptos_consensus_types::{order_vote::OrderVote, quorum_cert::QuorumCert};
    use aptos_crypto::{bls12381, HashValue};
    use aptos_types::{
        aggregate_signature::PartialSignatures, block_info::BlockInfo, epoch_state::EpochState,
        ledger_info::LedgerInfo, validator_verifier::random_validator_verifier,
    };
    use std::sync::Arc;

    /// Creates a random ledger info for epoch 1 and round 1.
    fn random_ledger_info() -> LedgerInfo {
        LedgerInfo::new(
            BlockInfo::new(1, 0, HashValue::random(), HashValue::random(), 0, 0, None),
            HashValue::random(),
        )
    }

    #[test]
    fn order_vote_aggregation() {
        ::aptos_logger::Logger::init_for_testing();
        // set up 4 validators
        let (signers, verifier) = random_validator_verifier(4, Some(2), false);
        let epoch_state = Arc::new(EpochState::new(1, verifier));

        let mut pending_order_votes = PendingOrderVotes::new();

        // create random vote from validator[0]
        let li1 = random_ledger_info();
        let qc = QuorumCert::dummy();
        let order_vote_1_author_0 = OrderVote::new_with_signature(
            signers[0].author(),
            li1.clone(),
            signers[0].sign(&li1).expect("Unable to sign ledger info"),
        );

        // first time a new order vote is added -> OrderVoteAdded
        order_vote_1_author_0.set_verified();
        assert_eq!(
            pending_order_votes.insert_order_vote(
                &order_vote_1_author_0,
                epoch_state.clone(),
                Some(qc.clone())
            ),
            OrderVoteReceptionResult::VoteAdded(1)
        );

        // same author voting for the same thing -> OrderVoteAdded
        assert_eq!(
            pending_order_votes.insert_order_vote(
                &order_vote_1_author_0,
                epoch_state.clone(),
                None
            ),
            OrderVoteReceptionResult::VoteAdded(1)
        );

        // same author voting for a different result -> EquivocateVote
        let li2 = random_ledger_info();
        let order_vote_2_author_1 = OrderVote::new_with_signature(
            signers[1].author(),
            li2.clone(),
            signers[1].sign(&li2).expect("Unable to sign ledger info"),
        );
        order_vote_2_author_1.set_verified();
        assert_eq!(
            pending_order_votes.insert_order_vote(
                &order_vote_2_author_1,
                epoch_state.clone(),
                Some(qc.clone())
            ),
            OrderVoteReceptionResult::VoteAdded(1)
        );

        assert!(!pending_order_votes.has_enough_order_votes(&li1));
        assert!(!pending_order_votes.has_enough_order_votes(&li2));

        let order_vote_2_author_2 = OrderVote::new_with_signature(
            signers[2].author(),
            li2.clone(),
            signers[2].sign(&li2).expect("Unable to sign ledger info"),
        );
        order_vote_2_author_2.set_verified();
        match pending_order_votes.insert_order_vote(
            &order_vote_2_author_2,
            epoch_state.clone(),
            None,
        ) {
            OrderVoteReceptionResult::NewLedgerInfoWithSignatures((_qc, li_with_sig)) => {
                assert!(li_with_sig
                    .check_voting_power(&epoch_state.verifier)
                    .is_ok());
            },
            _ => {
                panic!("No QC formed.");
            },
        };
        assert!(!pending_order_votes.has_enough_order_votes(&li1));
        assert!(pending_order_votes.has_enough_order_votes(&li2));

        pending_order_votes.garbage_collect(0);
        assert!(!pending_order_votes.has_enough_order_votes(&li1));
        assert!(!pending_order_votes.has_enough_order_votes(&li2));
    }

    #[test]
    fn order_vote_aggregation_with_unverified_votes() {
        ::aptos_logger::Logger::init_for_testing();

        let (signers, verifier) = random_validator_verifier(5, Some(3), false);
        let epoch_state = Arc::new(EpochState::new(1, verifier));
        let mut pending_order_votes = PendingOrderVotes::new();
        let mut partial_signatures = PartialSignatures::empty();
        let qc = QuorumCert::dummy();

        // create random vote from validator[0]
        let li = random_ledger_info();
        let vote_0 = OrderVote::new_with_signature(
            signers[0].author(),
            li.clone(),
            signers[0].sign(&li).expect("Unable to sign ledger info"),
        );
        partial_signatures.add_signature(signers[0].author(), vote_0.signature().clone());

        let vote_1 = OrderVote::new_with_signature(
            signers[1].author(),
            li.clone(),
            signers[1].sign(&li).expect("Unable to sign ledger info"),
        );
        partial_signatures.add_signature(signers[1].author(), vote_1.signature().clone());

        let vote_2 = OrderVote::new_with_signature(
            signers[2].author(),
            li.clone(),
            bls12381::Signature::dummy_signature(),
        );

        let vote_3 = OrderVote::new_with_signature(
            signers[3].author(),
            li.clone(),
            signers[3].sign(&li).expect("Unable to sign ledger info"),
        );
        partial_signatures.add_signature(signers[3].author(), vote_3.signature().clone());

        let vote_4 = OrderVote::new_with_signature(
            signers[4].author(),
            li.clone(),
            signers[4].sign(&li).expect("Unable to sign ledger info"),
        );

        assert_eq!(
            pending_order_votes.insert_order_vote(&vote_0, epoch_state.clone(), Some(qc.clone())),
            OrderVoteReceptionResult::VoteAdded(1)
        );

        vote_0.set_verified();
        assert_eq!(
            pending_order_votes.insert_order_vote(&vote_0, epoch_state.clone(), None),
            OrderVoteReceptionResult::VoteAdded(1)
        );

        vote_1.set_verified();
        assert_eq!(
            pending_order_votes.insert_order_vote(&vote_1, epoch_state.clone(), None),
            OrderVoteReceptionResult::VoteAdded(2)
        );

        assert_eq!(epoch_state.verifier.pessimistic_verify_set().len(), 0);
        assert_eq!(
            pending_order_votes.insert_order_vote(&vote_2, epoch_state.clone(), None),
            OrderVoteReceptionResult::VoteAdded(2)
        );
        assert_eq!(epoch_state.verifier.pessimistic_verify_set().len(), 1);

        let aggregate_sig = epoch_state
            .verifier
            .aggregate_signatures(partial_signatures.signatures_iter())
            .unwrap();
        match pending_order_votes.insert_order_vote(&vote_3, epoch_state.clone(), None) {
            OrderVoteReceptionResult::NewLedgerInfoWithSignatures((_qc, li_with_sig)) => {
                assert!(li_with_sig
                    .check_voting_power(&epoch_state.verifier)
                    .is_ok());

                assert_eq!(li_with_sig.signatures().clone(), aggregate_sig.clone());
            },
            _ => {
                panic!("No QC formed.");
            },
        };

        match pending_order_votes.insert_order_vote(&vote_4, epoch_state.clone(), None) {
            OrderVoteReceptionResult::NewLedgerInfoWithSignatures((_qc, li_with_sig)) => {
                assert!(li_with_sig
                    .check_voting_power(&epoch_state.verifier)
                    .is_ok());

                assert_eq!(li_with_sig.signatures().clone(), aggregate_sig.clone());
            },
            _ => {
                panic!("No QC formed.");
            },
        };
    }
}
