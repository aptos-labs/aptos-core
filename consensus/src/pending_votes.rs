// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! PendingVotes store pending votes observed for a fixed epoch and round.
//! It is meant to be used inside of a RoundState.
//! The module takes care of creating a QC or a TC
//! when enough votes (or timeout votes) have been observed.
//! Votes are automatically dropped when the structure goes out of scope.

use crate::counters;
use aptos_consensus_types::{
    common::Author,
    quorum_cert::QuorumCert,
    timeout_2chain::{TwoChainTimeoutCertificate, TwoChainTimeoutWithPartialSignatures},
    vote::Vote,
    vote_data::VoteData,
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_logger::prelude::*;
use aptos_types::{
    aggregate_signature::PartialSignatures,
    ledger_info::LedgerInfoWithPartialSignatures,
    validator_verifier::{ValidatorVerifier, VerifyError},
};
use std::{
    collections::{BTreeMap, HashMap},
    fmt,
    sync::Arc,
};

/// Result of the vote processing. The failure case (Verification error) is returned
/// as the Error part of the result.
#[derive(Debug, PartialEq, Eq)]
pub enum VoteReceptionResult {
    /// The vote has been added but QC has not been formed yet. Return the amount of voting power
    /// QC currently has.
    VoteAdded(u128),
    /// The very same vote message has been processed in past.
    DuplicateVote,
    /// The very same author has already voted for another proposal in this round (equivocation).
    EquivocateVote,
    /// This block has just been certified after adding the vote.
    NewQuorumCertificate(Arc<QuorumCert>),
    /// The vote completes a new TwoChainTimeoutCertificate
    New2ChainTimeoutCertificate(Arc<TwoChainTimeoutCertificate>),
    /// There might be some issues adding a vote
    ErrorAddingVote(VerifyError),
    /// Error happens when aggregating signature
    ErrorAggregatingSignature(VerifyError),
    /// Error happens when aggregating timeout certificated
    ErrorAggregatingTimeoutCertificate(VerifyError),
    /// The vote is not for the current round.
    UnexpectedRound(u64, u64),
    /// Receive f+1 timeout to trigger a local timeout, return the amount of voting power TC currently has.
    EchoTimeout(u128),
}

/// A PendingVotes structure keep track of votes
pub struct PendingVotes {
    /// Maps LedgerInfo digest to associated signatures (contained in a partial LedgerInfoWithSignatures).
    /// This might keep multiple LedgerInfos for the current round: either due to different proposals (byzantine behavior)
    /// or due to different NIL proposals (clients can have a different view of what block to extend).
    li_digest_to_votes:
        HashMap<HashValue /* LedgerInfo digest */, (usize, LedgerInfoWithPartialSignatures)>,
    /// Tracks all the signatures of the 2-chain timeout for the given round.
    maybe_partial_2chain_tc: Option<TwoChainTimeoutWithPartialSignatures>,
    /// Map of Author to (vote, li_digest). This is useful to discard multiple votes.
    author_to_vote: HashMap<Author, (Vote, HashValue)>,
    /// Whether we have echoed timeout for this round.
    echo_timeout: bool,
}

impl PendingVotes {
    /// Creates an empty PendingVotes structure for a specific epoch and round
    pub fn new() -> Self {
        PendingVotes {
            li_digest_to_votes: HashMap::new(),
            maybe_partial_2chain_tc: None,
            author_to_vote: HashMap::new(),
            echo_timeout: false,
        }
    }

    /// Insert a vote and if the vote is valid, return a QuorumCertificate preferentially over a
    /// TimeoutCertificate if either can can be formed
    pub fn insert_vote(
        &mut self,
        vote: &Vote,
        validator_verifier: &ValidatorVerifier,
    ) -> VoteReceptionResult {
        // derive data from vote
        let li_digest = vote.ledger_info().hash();

        //
        // 1. Has the author already voted for this round?
        //

        if let Some((previously_seen_vote, previous_li_digest)) =
            self.author_to_vote.get(&vote.author())
        {
            // is it the same vote?
            if &li_digest == previous_li_digest {
                // we've already seen an equivalent vote before
                let new_timeout_vote = vote.is_timeout() && !previously_seen_vote.is_timeout();
                if !new_timeout_vote {
                    // it's not a new timeout vote
                    return VoteReceptionResult::DuplicateVote;
                }
            } else {
                // we have seen a different vote for the same round
                error!(
                    SecurityEvent::ConsensusEquivocatingVote,
                    remote_peer = vote.author(),
                    vote = vote,
                    previous_vote = previously_seen_vote
                );

                return VoteReceptionResult::EquivocateVote;
            }
        }

        //
        // 2. Store new vote (or update, in case it's a new timeout vote)
        //

        self.author_to_vote
            .insert(vote.author(), (vote.clone(), li_digest));

        //
        // 3. Let's check if we can create a QC
        //

        let len = self.li_digest_to_votes.len() + 1;
        // obtain the ledger info with signatures associated to the vote's ledger info
        let (hash_index, li_with_sig) =
            self.li_digest_to_votes.entry(li_digest).or_insert_with(|| {
                // if the ledger info with signatures doesn't exist yet, create it
                (
                    len,
                    LedgerInfoWithPartialSignatures::new(
                        vote.ledger_info().clone(),
                        PartialSignatures::empty(),
                    ),
                )
            });

        let validator_voting_power = validator_verifier
            .get_voting_power(&vote.author())
            .unwrap_or(0);
        if validator_voting_power == 0 {
            warn!("Received vote with no voting power, from {}", vote.author());
        }
        let cur_epoch = vote.vote_data().proposed().epoch() as i64;
        let cur_round = vote.vote_data().proposed().round() as i64;
        counters::CONSENSUS_CURRENT_ROUND_QUORUM_VOTING_POWER
            .set(validator_verifier.quorum_voting_power() as f64);

        if !vote.is_timeout() {
            counters::CONSENSUS_CURRENT_ROUND_VOTED_POWER
                .with_label_values(&[&vote.author().to_string(), &hash_index_to_str(*hash_index)])
                .set(validator_voting_power as f64);
            counters::CONSENSUS_LAST_VOTE_EPOCH
                .with_label_values(&[&vote.author().to_string()])
                .set(cur_epoch);
            counters::CONSENSUS_LAST_VOTE_ROUND
                .with_label_values(&[&vote.author().to_string()])
                .set(cur_round);
        }

        // add this vote to the ledger info with signatures
        li_with_sig.add_signature(vote.author(), vote.signature().clone());

        // check if we have enough signatures to create a QC
        let voting_power = match validator_verifier
            .check_voting_power(li_with_sig.signatures().keys(), true)
        {
            // a quorum of signature was reached, a new QC is formed
            Ok(aggregated_voting_power) => {
                assert!(
                        aggregated_voting_power >= validator_verifier.quorum_voting_power(),
                        "QC aggregation should not be triggered if we don't have enough votes to form a QC"
                    );
                match li_with_sig.aggregate_signatures(validator_verifier) {
                    Ok(ledger_info_with_sig) => {
                        return VoteReceptionResult::NewQuorumCertificate(Arc::new(
                            QuorumCert::new(vote.vote_data().clone(), ledger_info_with_sig),
                        ))
                    },
                    Err(e) => return VoteReceptionResult::ErrorAggregatingSignature(e),
                }
            },

            // not enough votes
            Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => voting_power,

            // error
            Err(error) => {
                error!(
                    "MUST_FIX: vote received could not be added: {}, vote: {}",
                    error, vote
                );
                return VoteReceptionResult::ErrorAddingVote(error);
            },
        };

        //
        // 4. We couldn't form a QC, let's check if we can create a TC
        //

        if let Some((timeout, signature)) = vote.two_chain_timeout() {
            counters::CONSENSUS_CURRENT_ROUND_TIMEOUT_VOTED_POWER
                .with_label_values(&[&vote.author().to_string()])
                .set(validator_voting_power as f64);
            counters::CONSENSUS_LAST_TIMEOUT_VOTE_EPOCH
                .with_label_values(&[&vote.author().to_string()])
                .set(cur_epoch);
            counters::CONSENSUS_LAST_TIMEOUT_VOTE_ROUND
                .with_label_values(&[&vote.author().to_string()])
                .set(cur_round);

            let partial_tc = self
                .maybe_partial_2chain_tc
                .get_or_insert_with(|| TwoChainTimeoutWithPartialSignatures::new(timeout.clone()));
            partial_tc.add(vote.author(), timeout.clone(), signature.clone());
            let tc_voting_power =
                match validator_verifier.check_voting_power(partial_tc.signers(), true) {
                    Ok(_) => {
                        return match partial_tc.aggregate_signatures(validator_verifier) {
                            Ok(tc_with_sig) => VoteReceptionResult::New2ChainTimeoutCertificate(
                                Arc::new(tc_with_sig),
                            ),
                            Err(e) => VoteReceptionResult::ErrorAggregatingTimeoutCertificate(e),
                        };
                    },
                    Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => voting_power,
                    Err(error) => {
                        error!(
                        "MUST_FIX: 2-chain timeout vote received could not be added: {}, vote: {}",
                        error, vote
                    );
                        return VoteReceptionResult::ErrorAddingVote(error);
                    },
                };

            // Echo timeout if receive f+1 timeout message.
            if !self.echo_timeout {
                let f_plus_one = validator_verifier.total_voting_power()
                    - validator_verifier.quorum_voting_power()
                    + 1;
                if tc_voting_power >= f_plus_one {
                    self.echo_timeout = true;
                    return VoteReceptionResult::EchoTimeout(tc_voting_power);
                }
            }
        }

        //
        // 5. No QC (or TC) could be formed, return the QC's voting power
        //

        VoteReceptionResult::VoteAdded(voting_power)
    }

    pub fn aggregate_qc_now(
        validator_verifier: &ValidatorVerifier,
        li_with_sig: &LedgerInfoWithPartialSignatures,
        vote_data: &VoteData,
    ) -> VoteReceptionResult {
        match li_with_sig.aggregate_signatures(validator_verifier) {
            Ok(ledger_info_with_sig) => VoteReceptionResult::NewQuorumCertificate(Arc::new(
                QuorumCert::new(vote_data.clone(), ledger_info_with_sig),
            )),
            Err(e) => VoteReceptionResult::ErrorAggregatingSignature(e),
        }
    }

    pub fn process_delayed_qc(
        &mut self,
        validator_verifier: &ValidatorVerifier,
        vote: Vote,
    ) -> VoteReceptionResult {
        let li_digest = vote.ledger_info().hash();
        match self.li_digest_to_votes.get_mut(&li_digest) {
            Some((_, li_with_sig)) => {
                match validator_verifier.check_voting_power(li_with_sig.signatures().keys(), true) {
                    // a quorum of signature was reached, a new QC is formed
                    Ok(_) => {
                        Self::aggregate_qc_now(validator_verifier, li_with_sig, vote.vote_data())
                    },

                    // not enough votes
                    Err(VerifyError::TooLittleVotingPower { .. }) => {
                        panic!("Delayed QC aggregation should not be triggered if we don't have enough votes to form a QC");
                    },

                    // error
                    Err(error) => {
                        error!(
                            "MUST_FIX: vote received could not be added: {}, vote: {}",
                            error, vote
                        );
                        VoteReceptionResult::ErrorAddingVote(error)
                    },
                }
            },
            None => {
                error!(
                    "No LedgerInfoWithSignatures found for the given digest: {}",
                    li_digest
                );
                VoteReceptionResult::ErrorAddingVote(VerifyError::EmptySignature)
            },
        }
    }

    pub fn drain_votes(
        &mut self,
    ) -> (
        Vec<(HashValue, LedgerInfoWithPartialSignatures)>,
        Option<TwoChainTimeoutWithPartialSignatures>,
    ) {
        for (hash_index, _) in self.li_digest_to_votes.values() {
            let hash_index_str = hash_index_to_str(*hash_index);
            for author in self.author_to_vote.keys() {
                counters::CONSENSUS_CURRENT_ROUND_VOTED_POWER
                    .with_label_values(&[&author.to_string(), &hash_index_str])
                    .set(0_f64);
            }
        }
        if let Some(partial_tc) = &self.maybe_partial_2chain_tc {
            for author in partial_tc.signers() {
                counters::CONSENSUS_CURRENT_ROUND_TIMEOUT_VOTED_POWER
                    .with_label_values(&[&author.to_string()])
                    .set(0_f64);
            }
        }

        (
            self.li_digest_to_votes
                .drain()
                .map(|(key, (_, li))| (key, li))
                .collect(),
            self.maybe_partial_2chain_tc.take(),
        )
    }
}

fn hash_index_to_str(hash_index: usize) -> String {
    if hash_index <= 2 {
        hash_index.to_string()
    } else {
        "other".to_string()
    }
}

//
// Helpful trait implementation
//

impl fmt::Display for PendingVotes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // collect votes per ledger info
        let votes = self
            .li_digest_to_votes
            .iter()
            .map(|(li_digest, (_, li))| (li_digest, li.signatures().keys().collect::<Vec<_>>()))
            .collect::<BTreeMap<_, _>>();

        // collect timeout votes
        let timeout_votes = self
            .maybe_partial_2chain_tc
            .as_ref()
            .map(|partial_tc| partial_tc.signers().collect::<Vec<_>>());

        // write
        write!(f, "PendingVotes: [")?;

        for (hash, authors) in votes {
            write!(f, "LI {} has {} votes {:?} ", hash, authors.len(), authors)?;
        }

        if let Some(authors) = timeout_votes {
            write!(f, "{} timeout {:?}", authors.len(), authors)?;
        }

        write!(f, "]")
    }
}

//
// Tests
//

#[cfg(test)]
mod tests {
    use super::{PendingVotes, VoteReceptionResult};
    use aptos_consensus_types::{
        block::block_test_utils::certificate_for_genesis, vote::Vote, vote_data::VoteData,
    };
    use aptos_crypto::HashValue;
    use aptos_types::{
        block_info::BlockInfo, ledger_info::LedgerInfo,
        validator_verifier::random_validator_verifier,
    };
    use itertools::Itertools;

    /// Creates a random ledger info for epoch 1 and round 1.
    fn random_ledger_info() -> LedgerInfo {
        LedgerInfo::new(
            BlockInfo::new(1, 0, HashValue::random(), HashValue::random(), 0, 0, None),
            HashValue::random(),
        )
    }

    /// Creates a random VoteData for epoch 1 and round 1,
    /// extending a random block at epoch1 and round 0.
    fn random_vote_data() -> VoteData {
        VoteData::new(BlockInfo::random(1), BlockInfo::random(0))
    }

    #[test]
    /// Verify that votes are properly aggregated to QC based on their LedgerInfo digest
    fn test_qc_aggregation() {
        ::aptos_logger::Logger::init_for_testing();

        // set up 4 validators
        let (signers, validator) = random_validator_verifier(4, Some(2), false);
        let mut pending_votes = PendingVotes::new();

        // create random vote from validator[0]
        let li1 = random_ledger_info();
        let vote_data_1 = random_vote_data();
        let vote_data_1_author_0 =
            Vote::new(vote_data_1, signers[0].author(), li1, &signers[0]).unwrap();

        // first time a new vote is added -> VoteAdded
        assert_eq!(
            pending_votes.insert_vote(&vote_data_1_author_0, &validator),
            VoteReceptionResult::VoteAdded(1)
        );

        // same author voting for the same thing -> DuplicateVote
        assert_eq!(
            pending_votes.insert_vote(&vote_data_1_author_0, &validator),
            VoteReceptionResult::DuplicateVote
        );

        // same author voting for a different result -> EquivocateVote
        let li2 = random_ledger_info();
        let vote_data_2 = random_vote_data();
        let vote_data_2_author_0 = Vote::new(
            vote_data_2.clone(),
            signers[0].author(),
            li2.clone(),
            &signers[0],
        )
        .unwrap();
        assert_eq!(
            pending_votes.insert_vote(&vote_data_2_author_0, &validator),
            VoteReceptionResult::EquivocateVote
        );

        // a different author voting for a different result -> VoteAdded
        let vote_data_2_author_1 = Vote::new(
            vote_data_2.clone(),
            signers[1].author(),
            li2.clone(),
            &signers[1],
        )
        .unwrap();
        assert_eq!(
            pending_votes.insert_vote(&vote_data_2_author_1, &validator),
            VoteReceptionResult::VoteAdded(1)
        );

        // two votes for the ledger info -> NewQuorumCertificate
        let vote_data_2_author_2 =
            Vote::new(vote_data_2, signers[2].author(), li2, &signers[2]).unwrap();
        match pending_votes.insert_vote(&vote_data_2_author_2, &validator) {
            VoteReceptionResult::NewQuorumCertificate(qc) => {
                assert!(qc.ledger_info().check_voting_power(&validator).is_ok());
            },
            _ => {
                panic!("No QC formed.");
            },
        };
    }

    #[test]
    fn test_2chain_tc_aggregation() {
        ::aptos_logger::Logger::init_for_testing();

        // set up 4 validators
        let (signers, validator) = random_validator_verifier(4, None, false);
        let mut pending_votes = PendingVotes::new();

        // submit a new vote from validator[0] -> VoteAdded
        let li0 = random_ledger_info();
        let vote0 = random_vote_data();
        let mut vote0_author_0 = Vote::new(vote0, signers[0].author(), li0, &signers[0]).unwrap();

        assert_eq!(
            pending_votes.insert_vote(&vote0_author_0, &validator),
            VoteReceptionResult::VoteAdded(1)
        );

        // submit the same vote but enhanced with a timeout -> VoteAdded
        let timeout = vote0_author_0.generate_2chain_timeout(certificate_for_genesis());
        let signature = timeout.sign(&signers[0]).unwrap();
        vote0_author_0.add_2chain_timeout(timeout, signature);

        assert_eq!(
            pending_votes.insert_vote(&vote0_author_0, &validator),
            VoteReceptionResult::VoteAdded(1)
        );

        // another vote for a different block cannot form a TC if it doesn't have a timeout signature
        let li1 = random_ledger_info();
        let vote1 = random_vote_data();
        let mut vote1_author_1 = Vote::new(vote1, signers[1].author(), li1, &signers[1]).unwrap();
        assert_eq!(
            pending_votes.insert_vote(&vote1_author_1, &validator),
            VoteReceptionResult::VoteAdded(1)
        );

        // if that vote is now enhanced with a timeout signature -> EchoTimeout.
        let timeout = vote1_author_1.generate_2chain_timeout(certificate_for_genesis());
        let signature = timeout.sign(&signers[1]).unwrap();
        vote1_author_1.add_2chain_timeout(timeout, signature);
        match pending_votes.insert_vote(&vote1_author_1, &validator) {
            VoteReceptionResult::EchoTimeout(voting_power) => {
                assert_eq!(voting_power, 2);
            },
            _ => {
                panic!("Should echo timeout");
            },
        };

        let li2 = random_ledger_info();
        let vote2 = random_vote_data();
        let mut vote2_author_2 = Vote::new(vote2, signers[2].author(), li2, &signers[2]).unwrap();

        // if that vote is now enhanced with a timeout signature -> NewTimeoutCertificate.
        let timeout = vote2_author_2.generate_2chain_timeout(certificate_for_genesis());
        let signature = timeout.sign(&signers[2]).unwrap();
        vote2_author_2.add_2chain_timeout(timeout, signature);

        match pending_votes.insert_vote(&vote2_author_2, &validator) {
            VoteReceptionResult::New2ChainTimeoutCertificate(tc) => {
                assert!(validator
                    .check_voting_power(
                        tc.signatures_with_rounds()
                            .get_voters(
                                &validator.get_ordered_account_addresses_iter().collect_vec()
                            )
                            .iter(),
                        true
                    )
                    .is_ok());
            },
            _ => {
                panic!("Should form TC");
            },
        };
    }
}
