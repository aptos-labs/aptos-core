// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pending_votes::TwoChainTimeoutVotes;
use velor_bitvec::BitVec;
use velor_consensus_types::{
    quorum_cert::QuorumCert, round_timeout::RoundTimeoutReason, timeout_2chain::TwoChainTimeout,
};
use velor_types::validator_verifier::{
    random_validator_verifier, random_validator_verifier_with_voting_power,
};
use itertools::Itertools;

#[test]
fn test_two_chain_timeout_votes_aggregation() {
    let epoch = 1;
    let round = 10;
    let (signers, verifier) = random_validator_verifier(4, None, false);
    let all_reasons = [
        RoundTimeoutReason::NoQC,
        RoundTimeoutReason::ProposalNotReceived,
        RoundTimeoutReason::Unknown,
        RoundTimeoutReason::PayloadUnavailable {
            missing_authors: BitVec::with_num_bits(signers.len() as u16),
        },
    ];

    // Majority nodes timeout with same reason
    for reason in &all_reasons {
        let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
        let mut two_chain_timeout_votes = TwoChainTimeoutVotes::new(timeout);
        for signer in signers.iter().take(3) {
            let author = signer.author();
            let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
            let signature = signer.sign(&timeout.signing_format()).unwrap();
            two_chain_timeout_votes.add(author, timeout, signature, reason.clone());
        }
        let (_, aggregate_timeout_reason) = two_chain_timeout_votes.unpack_aggregate(&verifier);
        assert_eq!(aggregate_timeout_reason, reason.clone());
    }

    // Minority nodes timeout with same reason and one with different reason
    for permut in all_reasons.iter().permutations(2) {
        let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
        let mut two_chain_timeout_votes = TwoChainTimeoutVotes::new(timeout);
        for signer in signers.iter().take(2) {
            let author = signer.author();
            let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
            let signature = signer.sign(&timeout.signing_format()).unwrap();
            two_chain_timeout_votes.add(author, timeout, signature, permut[0].clone());
        }

        let author = signers[2].author();
        let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
        let signature = signers[2].sign(&timeout.signing_format()).unwrap();
        two_chain_timeout_votes.add(author, timeout, signature, permut[1].clone());

        let (_, aggregate_timeout_reason) = two_chain_timeout_votes.unpack_aggregate(&verifier);
        assert_eq!(aggregate_timeout_reason, permut[0].clone());
    }
}

#[test]
fn test_two_chain_timeout_aggregate_missing_authors() {
    let epoch = 1;
    let round = 10;
    let (signers, verifier) =
        random_validator_verifier_with_voting_power(4, None, false, &[3, 3, 2, 1]);

    let permutations = [true, true, false, false]
        .iter()
        .copied()
        .permutations(4)
        .unique();

    // Minority nodes report the same set of missing authors
    for permut in permutations {
        let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
        let mut two_chain_timeout_votes = TwoChainTimeoutVotes::new(timeout);
        for signer in signers.iter().take(2) {
            let author = signer.author();
            let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
            let signature = signer.sign(&timeout.signing_format()).unwrap();
            let reason = RoundTimeoutReason::PayloadUnavailable {
                missing_authors: permut.clone().into(),
            };
            two_chain_timeout_votes.add(author, timeout, signature, reason);
        }

        let author = signers[2].author();
        let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
        let signature = signers[2].sign(&timeout.signing_format()).unwrap();
        two_chain_timeout_votes.add(author, timeout, signature, RoundTimeoutReason::Unknown);

        let (_, aggregate_timeout_reason) = two_chain_timeout_votes.unpack_aggregate(&verifier);

        assert_eq!(
            aggregate_timeout_reason,
            RoundTimeoutReason::PayloadUnavailable {
                missing_authors: permut.clone().into()
            }
        );
    }

    // Not enough votes to form a valid timeout reason
    let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
    let mut two_chain_timeout_votes = TwoChainTimeoutVotes::new(timeout);

    let author = signers[2].author();
    let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
    let signature = signers[2].sign(&timeout.signing_format()).unwrap();
    two_chain_timeout_votes.add(
        author,
        timeout,
        signature,
        RoundTimeoutReason::PayloadUnavailable {
            missing_authors: vec![true, false, false, false].into(),
        },
    );

    let (_, aggregate_timeout_reason) = two_chain_timeout_votes.unpack_aggregate(&verifier);

    assert_eq!(aggregate_timeout_reason, RoundTimeoutReason::Unknown);

    // Not enough nodes vote for the same node.
    let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
    let mut two_chain_timeout_votes = TwoChainTimeoutVotes::new(timeout);

    let author = signers[2].author();
    let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
    let signature = signers[2].sign(&timeout.signing_format()).unwrap();
    two_chain_timeout_votes.add(
        author,
        timeout,
        signature,
        RoundTimeoutReason::PayloadUnavailable {
            missing_authors: vec![false, true, false, false].into(),
        },
    );

    let author = signers[3].author();
    let timeout = TwoChainTimeout::new(epoch, round, QuorumCert::dummy());
    let signature = signers[3].sign(&timeout.signing_format()).unwrap();
    two_chain_timeout_votes.add(
        author,
        timeout,
        signature,
        RoundTimeoutReason::PayloadUnavailable {
            missing_authors: vec![false, false, false, true].into(),
        },
    );

    let (_, aggregate_timeout_reason) = two_chain_timeout_votes.unpack_aggregate(&verifier);

    assert_eq!(
        aggregate_timeout_reason,
        RoundTimeoutReason::PayloadUnavailable {
            missing_authors: BitVec::with_num_bits(4)
        }
    );
}
