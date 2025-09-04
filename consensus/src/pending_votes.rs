// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! PendingVotes store pending votes observed for a fixed epoch and round.
//! It is meant to be used inside of a RoundState.
//! The module takes care of creating a QC or a TC
//! when enough votes (or timeout votes) have been observed.
//! Votes are automatically dropped when the structure goes out of scope.

use crate::counters;
use velor_bitvec::BitVec;
use velor_consensus_types::{
    common::Author,
    quorum_cert::QuorumCert,
    round_timeout::{RoundTimeout, RoundTimeoutReason},
    timeout_2chain::{
        TwoChainTimeout, TwoChainTimeoutCertificate, TwoChainTimeoutWithPartialSignatures,
    },
    vote::Vote,
};
use velor_crypto::{bls12381, hash::CryptoHash, HashValue};
use velor_logger::prelude::*;
use velor_types::{
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures, SignatureAggregator},
    validator_verifier::{ValidatorVerifier, VerifyError},
};
use std::{collections::HashMap, fmt, sync::Arc};

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
    /// The author of the vote is unknown
    UnknownAuthor(Author),
}

#[derive(Debug, PartialEq, Eq)]
pub enum VoteStatus {
    EnoughVotes(LedgerInfoWithSignatures),
    NotEnoughVotes(SignatureAggregator<LedgerInfo>),
}

#[derive(Debug)]
pub(super) struct TwoChainTimeoutVotes {
    timeout_reason: HashMap<Author, RoundTimeoutReason>,
    partial_2chain_tc: TwoChainTimeoutWithPartialSignatures,
}

impl TwoChainTimeoutVotes {
    pub(super) fn new(timeout: TwoChainTimeout) -> Self {
        Self {
            partial_2chain_tc: TwoChainTimeoutWithPartialSignatures::new(timeout.clone()),
            timeout_reason: HashMap::new(),
        }
    }

    pub(super) fn add(
        &mut self,
        author: Author,
        timeout: TwoChainTimeout,
        signature: bls12381::Signature,
        reason: RoundTimeoutReason,
    ) {
        self.partial_2chain_tc.add(author, timeout, signature);
        self.timeout_reason.entry(author).or_insert(reason);
    }

    pub(super) fn partial_2chain_tc_mut(&mut self) -> &mut TwoChainTimeoutWithPartialSignatures {
        &mut self.partial_2chain_tc
    }

    fn aggregated_timeout_reason(&self, verifier: &ValidatorVerifier) -> RoundTimeoutReason {
        let mut reason_voting_power: HashMap<RoundTimeoutReason, u128> = HashMap::new();
        let mut missing_batch_authors: HashMap<usize, u128> = HashMap::new();
        // let ordered_authors = verifier.get_ordered_account_addresses();
        for (author, reason) in &self.timeout_reason {
            // To aggregate the reason, we only care about the variant type itself and
            // exclude any data within the variants.
            let reason_key = match reason {
                reason @ RoundTimeoutReason::Unknown
                | reason @ RoundTimeoutReason::ProposalNotReceived
                | reason @ RoundTimeoutReason::NoQC => reason.clone(),
                RoundTimeoutReason::PayloadUnavailable { missing_authors } => {
                    for missing_idx in missing_authors.iter_ones() {
                        *missing_batch_authors.entry(missing_idx).or_default() +=
                            verifier.get_voting_power(author).unwrap_or_default() as u128;
                    }
                    RoundTimeoutReason::PayloadUnavailable {
                        // Since we care only about the variant type, we replace the bitvec
                        // with a placeholder.
                        missing_authors: BitVec::with_num_bits(verifier.len() as u16),
                    }
                },
            };
            *reason_voting_power.entry(reason_key).or_default() +=
                verifier.get_voting_power(author).unwrap_or_default() as u128;
        }
        // The aggregated timeout reason is the reason with the most voting power received from
        // at least f+1 peers by voting power. If such voting power does not exist, then the
        // reason is unknown.

        reason_voting_power
            .into_iter()
            .max_by_key(|(_, voting_power)| *voting_power)
            .filter(|(_, voting_power)| {
                verifier
                    .check_aggregated_voting_power(*voting_power, false)
                    .is_ok()
            })
            .map(|(reason, _)| {
                // If the aggregated reason is due to unavailable payload, we will compute the
                // aggregated missing authors bitvec counting batch authors that have been reported
                // missing by minority peers.
                if matches!(reason, RoundTimeoutReason::PayloadUnavailable { .. }) {
                    let mut aggregated_bitvec = BitVec::with_num_bits(verifier.len() as u16);
                    for (author_idx, voting_power) in missing_batch_authors {
                        if verifier
                            .check_aggregated_voting_power(voting_power, false)
                            .is_ok()
                        {
                            aggregated_bitvec.set(author_idx as u16);
                        }
                    }
                    RoundTimeoutReason::PayloadUnavailable {
                        missing_authors: aggregated_bitvec,
                    }
                } else {
                    reason
                }
            })
            .unwrap_or(RoundTimeoutReason::Unknown)
    }

    pub(crate) fn unpack_aggregate(
        self,
        verifier: &ValidatorVerifier,
    ) -> (TwoChainTimeoutWithPartialSignatures, RoundTimeoutReason) {
        let aggregated_reason = self.aggregated_timeout_reason(verifier);
        (self.partial_2chain_tc, aggregated_reason)
    }
}

/// A PendingVotes structure keep track of votes
pub struct PendingVotes {
    /// Maps LedgerInfo digest to associated signatures.
    /// This might keep multiple LedgerInfos for the current round: either due to different proposals (byzantine behavior)
    /// or due to different NIL proposals (clients can have a different view of what block to extend).
    li_digest_to_votes: HashMap<HashValue /* LedgerInfo digest */, (usize, VoteStatus)>,
    /// Tracks all the signatures of the 2-chain timeout for the given round.
    maybe_2chain_timeout_votes: Option<TwoChainTimeoutVotes>,
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
            maybe_2chain_timeout_votes: None,
            author_to_vote: HashMap::new(),
            echo_timeout: false,
        }
    }

    /// Insert a RoundTimeout and return a TimeoutCertificate if it can be formed
    pub fn insert_round_timeout(
        &mut self,
        round_timeout: &RoundTimeout,
        validator_verifier: &ValidatorVerifier,
    ) -> VoteReceptionResult {
        //
        // Let's check if we can create a TC
        //

        let timeout = round_timeout.two_chain_timeout();
        let signature = round_timeout.signature();

        let validator_voting_power = validator_verifier
            .get_voting_power(&round_timeout.author())
            .unwrap_or(0);
        if validator_voting_power == 0 {
            warn!(
                "Received vote with no voting power, from {}",
                round_timeout.author()
            );
        }
        let cur_epoch = round_timeout.epoch();
        let cur_round = round_timeout.round();

        counters::CONSENSUS_CURRENT_ROUND_TIMEOUT_VOTED_POWER
            .with_label_values(&[&round_timeout.author().to_string()])
            .set(validator_voting_power as f64);
        counters::CONSENSUS_LAST_TIMEOUT_VOTE_EPOCH
            .with_label_values(&[&round_timeout.author().to_string()])
            .set(cur_epoch as i64);
        counters::CONSENSUS_LAST_TIMEOUT_VOTE_ROUND
            .with_label_values(&[&round_timeout.author().to_string()])
            .set(cur_round as i64);

        let two_chain_votes = self
            .maybe_2chain_timeout_votes
            .get_or_insert_with(|| TwoChainTimeoutVotes::new(timeout.clone()));
        two_chain_votes.add(
            round_timeout.author(),
            timeout.clone(),
            signature.clone(),
            round_timeout.reason().clone(),
        );

        let partial_tc = two_chain_votes.partial_2chain_tc_mut();
        let tc_voting_power =
            match validator_verifier.check_voting_power(partial_tc.signers(), true) {
                Ok(_) => {
                    return match partial_tc.aggregate_signatures(validator_verifier) {
                        Ok(tc_with_sig) => {
                            VoteReceptionResult::New2ChainTimeoutCertificate(Arc::new(tc_with_sig))
                        },
                        Err(e) => VoteReceptionResult::ErrorAggregatingTimeoutCertificate(e),
                    };
                },
                Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => voting_power,
                Err(error) => {
                    error!(
                        "MUST_FIX: 2-chain timeout vote received could not be added: {}, vote: {}",
                        error, timeout
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

        //
        // No TC could be formed, return the TC's voting power
        //

        VoteReceptionResult::VoteAdded(tc_voting_power)
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
        let (hash_index, status) = self.li_digest_to_votes.entry(li_digest).or_insert_with(|| {
            (
                len,
                VoteStatus::NotEnoughVotes(SignatureAggregator::new(vote.ledger_info().clone())),
            )
        });

        let validator_voting_power = validator_verifier.get_voting_power(&vote.author());

        if validator_voting_power.is_none() {
            warn!("Received vote from an unknown author: {}", vote.author());
            return VoteReceptionResult::UnknownAuthor(vote.author());
        }
        let validator_voting_power =
            validator_voting_power.expect("Author must exist in the validator set.");
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

        let voting_power = match status {
            VoteStatus::EnoughVotes(li_with_sig) => {
                return VoteReceptionResult::NewQuorumCertificate(Arc::new(QuorumCert::new(
                    vote.vote_data().clone(),
                    li_with_sig.clone(),
                )));
            },
            VoteStatus::NotEnoughVotes(sig_aggregator) => {
                // add this vote to the signature aggregator
                sig_aggregator.add_signature(vote.author(), vote.signature_with_status());

                // check if we have enough signatures to create a QC
                match sig_aggregator.check_voting_power(validator_verifier, true) {
                    // a quorum of signature was reached, a new QC is formed
                    Ok(aggregated_voting_power) => {
                        assert!(
                                aggregated_voting_power >= validator_verifier.quorum_voting_power(),
                                "QC aggregation should not be triggered if we don't have enough votes to form a QC"
                            );
                        let verification_result = {
                            let _timer = counters::VERIFY_MSG
                                .with_label_values(&["vote_aggregate_and_verify"])
                                .start_timer();

                            sig_aggregator.aggregate_and_verify(validator_verifier).map(
                                |(ledger_info, aggregated_sig)| {
                                    LedgerInfoWithSignatures::new(ledger_info, aggregated_sig)
                                },
                            )
                        };
                        match verification_result {
                            Ok(ledger_info_with_sig) => {
                                *status = VoteStatus::EnoughVotes(ledger_info_with_sig.clone());
                                return VoteReceptionResult::NewQuorumCertificate(Arc::new(
                                    QuorumCert::new(vote.vote_data().clone(), ledger_info_with_sig),
                                ));
                            },
                            Err(VerifyError::TooLittleVotingPower { voting_power, .. }) => {
                                voting_power
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
                }
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

            let two_chain_votes = self
                .maybe_2chain_timeout_votes
                .get_or_insert_with(|| TwoChainTimeoutVotes::new(timeout.clone()));
            two_chain_votes.add(
                vote.author(),
                timeout.clone(),
                signature.clone(),
                RoundTimeoutReason::Unknown,
            );

            let partial_tc = two_chain_votes.partial_2chain_tc_mut();
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

    pub fn drain_votes(&mut self) -> (Vec<(HashValue, VoteStatus)>, Option<TwoChainTimeoutVotes>) {
        for (hash_index, _) in self.li_digest_to_votes.values() {
            let hash_index_str = hash_index_to_str(*hash_index);
            for author in self.author_to_vote.keys() {
                counters::CONSENSUS_CURRENT_ROUND_VOTED_POWER
                    .with_label_values(&[&author.to_string(), &hash_index_str])
                    .set(0_f64);
            }
        }
        if let Some(votes) = &self.maybe_2chain_timeout_votes {
            for author in votes.partial_2chain_tc.signers() {
                counters::CONSENSUS_CURRENT_ROUND_TIMEOUT_VOTED_POWER
                    .with_label_values(&[&author.to_string()])
                    .set(0_f64);
            }
        }

        (
            self.li_digest_to_votes
                .drain()
                .map(|(key, (_, vote_status))| (key, vote_status))
                .collect(),
            self.maybe_2chain_timeout_votes.take(),
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
        write!(f, "PendingVotes: [")?;

        for (li_digest, (_, status)) in self.li_digest_to_votes.iter() {
            match status {
                VoteStatus::EnoughVotes(_li) => {
                    write!(f, "LI {} has aggregated QC", li_digest)?;
                },
                VoteStatus::NotEnoughVotes(sig_aggregator) => {
                    write!(
                        f,
                        "LI {} has {} verified votes, {} unverified votes",
                        li_digest,
                        sig_aggregator.verified_voters().count(),
                        sig_aggregator.unverified_voters().count(),
                    )?;
                },
            }
        }

        // collect timeout votes
        let timeout_votes = self
            .maybe_2chain_timeout_votes
            .as_ref()
            .map(|votes| votes.partial_2chain_tc.signers().collect::<Vec<_>>());

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
    use super::{PendingVotes, VoteReceptionResult, VoteStatus};
    use velor_consensus_types::{
        block::block_test_utils::certificate_for_genesis, vote::Vote, vote_data::VoteData,
    };
    use velor_crypto::{bls12381, hash::CryptoHash, HashValue};
    use velor_types::{
        aggregate_signature::PartialSignatures, block_info::BlockInfo, ledger_info::LedgerInfo,
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
        ::velor_logger::Logger::init_for_testing();

        // set up 4 validators
        let (signers, validator_verifier) = random_validator_verifier(4, Some(2), false);
        let mut pending_votes = PendingVotes::new();

        // create random vote from validator[0]
        let li1 = random_ledger_info();
        let vote_data_1 = random_vote_data();
        let vote_data_1_author_0 =
            Vote::new(vote_data_1, signers[0].author(), li1, &signers[0]).unwrap();

        // first time a new vote is added -> VoteAdded
        assert_eq!(
            pending_votes.insert_vote(&vote_data_1_author_0, &validator_verifier),
            VoteReceptionResult::VoteAdded(1)
        );

        // same author voting for the same thing -> DuplicateVote
        assert_eq!(
            pending_votes.insert_vote(&vote_data_1_author_0, &validator_verifier),
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
            pending_votes.insert_vote(&vote_data_2_author_0, &validator_verifier),
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
            pending_votes.insert_vote(&vote_data_2_author_1, &validator_verifier),
            VoteReceptionResult::VoteAdded(1)
        );

        // two votes for the ledger info -> NewQuorumCertificate
        let vote_data_2_author_2 =
            Vote::new(vote_data_2, signers[2].author(), li2, &signers[2]).unwrap();
        match pending_votes.insert_vote(&vote_data_2_author_2, &validator_verifier) {
            VoteReceptionResult::NewQuorumCertificate(qc) => {
                assert!(qc
                    .ledger_info()
                    .check_voting_power(&validator_verifier)
                    .is_ok());
            },
            _ => {
                panic!("No QC formed.");
            },
        };
    }

    #[test]
    fn test_qc_aggregation_with_unverified_votes() {
        ::velor_logger::Logger::init_for_testing();

        // set up 4 validators
        let (signers, validator_verifier) = random_validator_verifier(7, Some(3), false);
        let mut pending_votes = PendingVotes::new();

        // create random vote from validator[0]
        let mut li = random_ledger_info();
        let vote_data = random_vote_data();
        li.set_consensus_data_hash(vote_data.hash());
        let li_hash = li.hash();

        let mut partial_sigs = PartialSignatures::empty();

        let vote_0 = Vote::new(
            vote_data.clone(),
            signers[0].author(),
            li.clone(),
            &signers[0],
        )
        .unwrap();

        let vote_1 = Vote::new(
            vote_data.clone(),
            signers[1].author(),
            li.clone(),
            &signers[1],
        )
        .unwrap();

        let vote_2 = Vote::new_with_signature(
            vote_data.clone(),
            signers[2].author(),
            li.clone(),
            bls12381::Signature::dummy_signature(),
        );

        let vote_3 = Vote::new(
            vote_data.clone(),
            signers[3].author(),
            li.clone(),
            &signers[3],
        )
        .unwrap();

        let vote_4 = Vote::new(
            vote_data.clone(),
            signers[4].author(),
            li.clone(),
            &signers[4],
        )
        .unwrap();

        // first time a new vote is added -> VoteAdded
        assert_eq!(
            pending_votes.insert_vote(&vote_0, &validator_verifier),
            VoteReceptionResult::VoteAdded(1)
        );
        partial_sigs.add_signature(signers[0].author(), vote_0.signature().clone());

        // same author voting for the same thing -> DuplicateVote
        assert_eq!(
            pending_votes.insert_vote(&vote_0, &validator_verifier),
            VoteReceptionResult::DuplicateVote
        );

        assert_eq!(
            pending_votes.insert_vote(&vote_1, &validator_verifier),
            VoteReceptionResult::VoteAdded(2)
        );
        partial_sigs.add_signature(signers[1].author(), vote_1.signature().clone());

        assert_eq!(validator_verifier.pessimistic_verify_set().len(), 0);

        assert_eq!(
            pending_votes.insert_vote(&vote_2, &validator_verifier),
            VoteReceptionResult::VoteAdded(2)
        );

        assert_eq!(validator_verifier.pessimistic_verify_set().len(), 1);
        let (_, vote_status) = pending_votes.li_digest_to_votes.get(&li_hash).unwrap();
        match vote_status {
            VoteStatus::NotEnoughVotes(sig_aggregator) => {
                assert_eq!(sig_aggregator.verified_voters().count(), 2);
                assert_eq!(sig_aggregator.unverified_voters().count(), 0);
            },
            _ => {
                panic!("QC should not be formed yet.");
            },
        }

        partial_sigs.add_signature(signers[3].author(), vote_3.signature().clone());
        let aggregated_sig = validator_verifier
            .aggregate_signatures(partial_sigs.signatures_iter())
            .unwrap();
        match pending_votes.insert_vote(&vote_3, &validator_verifier) {
            VoteReceptionResult::NewQuorumCertificate(qc) => {
                assert!(qc
                    .ledger_info()
                    .check_voting_power(&validator_verifier)
                    .is_ok());
                assert_eq!(
                    qc.ledger_info().signatures().clone(),
                    aggregated_sig.clone()
                );
            },
            _ => {
                panic!("No QC formed.");
            },
        };

        match pending_votes.insert_vote(&vote_4, &validator_verifier) {
            VoteReceptionResult::NewQuorumCertificate(qc) => {
                assert!(qc
                    .ledger_info()
                    .check_voting_power(&validator_verifier)
                    .is_ok());
                assert_eq!(
                    qc.ledger_info().signatures().clone(),
                    aggregated_sig.clone()
                );
            },
            _ => {
                panic!("No QC formed.");
            },
        };

        assert_eq!(validator_verifier.pessimistic_verify_set().len(), 1);
    }

    #[test]
    fn test_2chain_tc_aggregation() {
        ::velor_logger::Logger::init_for_testing();

        // set up 4 validators
        let (signers, validator_verifier) = random_validator_verifier(4, None, false);
        let mut pending_votes = PendingVotes::new();

        // submit a new vote from validator[0] -> VoteAdded
        let li0 = random_ledger_info();
        let vote0 = random_vote_data();
        let mut vote0_author_0 = Vote::new(vote0, signers[0].author(), li0, &signers[0]).unwrap();

        assert_eq!(
            pending_votes.insert_vote(&vote0_author_0, &validator_verifier),
            VoteReceptionResult::VoteAdded(1)
        );

        // submit the same vote but enhanced with a timeout -> VoteAdded
        let timeout = vote0_author_0.generate_2chain_timeout(certificate_for_genesis());
        let signature = timeout.sign(&signers[0]).unwrap();
        vote0_author_0.add_2chain_timeout(timeout, signature);

        assert_eq!(
            pending_votes.insert_vote(&vote0_author_0, &validator_verifier),
            VoteReceptionResult::VoteAdded(1)
        );

        // another vote for a different block cannot form a TC if it doesn't have a timeout signature
        let li1 = random_ledger_info();
        let vote1 = random_vote_data();
        let mut vote1_author_1 = Vote::new(vote1, signers[1].author(), li1, &signers[1]).unwrap();
        assert_eq!(
            pending_votes.insert_vote(&vote1_author_1, &validator_verifier),
            VoteReceptionResult::VoteAdded(1)
        );

        // if that vote is now enhanced with a timeout signature -> EchoTimeout.
        let timeout = vote1_author_1.generate_2chain_timeout(certificate_for_genesis());
        let signature = timeout.sign(&signers[1]).unwrap();
        vote1_author_1.add_2chain_timeout(timeout, signature);
        match pending_votes.insert_vote(&vote1_author_1, &validator_verifier) {
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
        match pending_votes.insert_vote(&vote2_author_2, &validator_verifier) {
            VoteReceptionResult::New2ChainTimeoutCertificate(tc) => {
                assert!(validator_verifier
                    .check_voting_power(
                        tc.signatures_with_rounds()
                            .get_voters(
                                &validator_verifier
                                    .get_ordered_account_addresses_iter()
                                    .collect_vec()
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
