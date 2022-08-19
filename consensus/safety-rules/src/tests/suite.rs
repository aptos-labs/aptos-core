// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{test_utils, test_utils::make_timeout_cert, Error, TSafetyRules};
use aptos_crypto::hash::{HashValue, ACCUMULATOR_PLACEHOLDER_HASH};
use aptos_types::aggregate_signature::AggregateSignature;
use aptos_types::{
    block_info::BlockInfo,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use consensus_types::{
    block::block_test_utils::random_payload,
    common::{Payload, Round},
    quorum_cert::QuorumCert,
    timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate},
    vote_proposal::VoteProposal,
};

type Proof = test_utils::Proof;

fn make_proposal_with_qc_and_proof(
    round: Round,
    proof: Proof,
    qc: QuorumCert,
    signer: &ValidatorSigner,
) -> VoteProposal {
    test_utils::make_proposal_with_qc_and_proof(Payload::empty(), round, proof, qc, signer)
}

fn make_proposal_with_parent(
    round: Round,
    parent: &VoteProposal,
    committed: Option<&VoteProposal>,
    signer: &ValidatorSigner,
) -> VoteProposal {
    test_utils::make_proposal_with_parent(Payload::empty(), round, parent, committed, signer)
}

pub type Callback = Box<dyn Fn() -> (Box<dyn TSafetyRules + Send + Sync>, ValidatorSigner)>;

pub fn run_test_suite(safety_rules: &Callback) {
    test_end_to_end(safety_rules);
    test_initialize(safety_rules);
    test_voting_bad_epoch(safety_rules);
    test_sign_old_proposal(safety_rules);
    test_sign_proposal_with_bad_signer(safety_rules);
    test_sign_proposal_with_invalid_qc(safety_rules);
    test_sign_proposal_with_early_preferred_round(safety_rules);
    test_uninitialized_signer(safety_rules);
    test_validator_not_in_set(safety_rules);
    test_key_not_in_store(safety_rules);
    test_2chain_rules(safety_rules);
    test_2chain_timeout(safety_rules);
    test_sign_commit_vote(safety_rules);
    test_bad_execution_output(safety_rules);
}

fn test_bad_execution_output(safety_rules: &Callback) {
    // build a tree of the following form:
    //                 _____
    //                /     \
    // genesis---a1--a2--a3  evil_a3
    //
    // evil_a3 attempts to append to a1 but fails append only check
    // a3 works as it properly extends a2
    let (mut safety_rules, signer) = safety_rules();

    let (proof, genesis_qc) = test_utils::make_genesis(&signer);
    let round = genesis_qc.certified_block().round();

    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc, &signer);
    let a2 = make_proposal_with_parent(round + 2, &a1, None, &signer);
    let a3 = make_proposal_with_parent(round + 3, &a2, None, &signer);

    safety_rules.initialize(&proof).unwrap();
    let a1_output = a1
        .accumulator_extension_proof()
        .verify(
            a1.block()
                .quorum_cert()
                .certified_block()
                .executed_state_id(),
        )
        .unwrap();

    let evil_proof = Proof::new(
        a1_output.frozen_subtree_roots().clone(),
        a1_output.num_leaves() + 1,
        vec![],
    );

    let evil_a3 = make_proposal_with_qc_and_proof(
        round + 3,
        evil_proof,
        a3.block().quorum_cert().clone(),
        &signer,
    );

    let evil_a3_block = safety_rules.construct_and_sign_vote_two_chain(&evil_a3, None);

    assert!(matches!(
        evil_a3_block.unwrap_err(),
        Error::InvalidAccumulatorExtension(_)
    ));

    let a3_block = safety_rules.construct_and_sign_vote_two_chain(&a3, None);
    a3_block.unwrap();
}

fn test_end_to_end(safety_rules: &Callback) {
    let (mut safety_rules, signer) = safety_rules();

    let (proof, genesis_qc) = test_utils::make_genesis(&signer);
    let round = genesis_qc.certified_block().round();

    let data = random_payload(2048);

    let p0 = test_utils::make_proposal_with_qc(round + 1, genesis_qc.clone(), &signer);
    let p1 = test_utils::make_proposal_with_parent(data.clone(), round + 2, &p0, None, &signer);
    let p2 = test_utils::make_proposal_with_parent(data.clone(), round + 3, &p1, None, &signer);
    let p3 = test_utils::make_proposal_with_parent(data, round + 4, &p2, Some(&p0), &signer);

    let state = safety_rules.consensus_state().unwrap();
    assert_eq!(
        state.last_voted_round(),
        genesis_qc.certified_block().round()
    );
    assert_eq!(
        state.preferred_round(),
        genesis_qc.certified_block().round()
    );

    safety_rules.initialize(&proof).unwrap();
    safety_rules
        .construct_and_sign_vote_two_chain(&p0, None)
        .unwrap();
    safety_rules
        .construct_and_sign_vote_two_chain(&p1, None)
        .unwrap();
    safety_rules
        .construct_and_sign_vote_two_chain(&p2, None)
        .unwrap();
    safety_rules
        .construct_and_sign_vote_two_chain(&p3, None)
        .unwrap();

    let state = safety_rules.consensus_state().unwrap();
    assert_eq!(state.last_voted_round(), round + 4);
    assert_eq!(state.preferred_round(), round + 2);
}

/// Initialize from scratch, ensure that SafetyRules can properly initialize from a Waypoint and
/// that it rejects invalid LedgerInfos or those that do not match.
fn test_initialize(safety_rules: &Callback) {
    let (mut safety_rules, signer) = safety_rules();

    let state = safety_rules.consensus_state().unwrap();
    assert_eq!(state.last_voted_round(), 0);
    assert_eq!(state.preferred_round(), 0);
    assert_eq!(state.epoch(), 1);

    let (proof, _genesis_qc) = test_utils::make_genesis(&signer);
    safety_rules.initialize(&proof).unwrap();

    let signer1 = ValidatorSigner::from_int(1);
    let (bad_proof, _bad_genesis_qc) = test_utils::make_genesis(&signer1);

    match safety_rules.initialize(&bad_proof) {
        Err(Error::InvalidEpochChangeProof(_)) => (),
        _ => panic!("Unexpected output"),
    };
}

fn test_voting_bad_epoch(safety_rules: &Callback) {
    // Test to verify epoch is the same between parent and proposed in a vote proposal
    // genesis--a1 -> a2 fails due to jumping to a different epoch
    let (mut safety_rules, signer) = safety_rules();

    let (proof, genesis_qc) = test_utils::make_genesis(&signer);
    let round = genesis_qc.certified_block().round();

    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc, &signer);
    let a2 = test_utils::make_proposal_with_parent_and_overrides(
        Payload::empty(),
        round + 3,
        &a1,
        None,
        &signer,
        Some(21),
        None,
    );
    safety_rules.initialize(&proof).unwrap();
    safety_rules
        .construct_and_sign_vote_two_chain(&a1, None)
        .unwrap();

    assert_eq!(
        safety_rules.construct_and_sign_vote_two_chain(&a2, None),
        Err(Error::IncorrectEpoch(21, 1))
    );
}

fn test_sign_old_proposal(safety_rules: &Callback) {
    // Test to sign a proposal which makes no progress, compared with last voted round

    let (mut safety_rules, signer) = safety_rules();

    let (proof, genesis_qc) = test_utils::make_genesis(&signer);
    let round = genesis_qc.certified_block().round();
    safety_rules.initialize(&proof).unwrap();

    let a1 = test_utils::make_proposal_with_qc(round, genesis_qc, &signer);
    let err = safety_rules
        .sign_proposal(a1.block().block_data())
        .unwrap_err();
    assert!(matches!(err, Error::InvalidProposal(_)));
}

fn test_sign_proposal_with_bad_signer(safety_rules: &Callback) {
    // Test to sign a proposal signed by an unrecognizable signer

    let (mut safety_rules, signer) = safety_rules();

    let (proof, genesis_qc) = test_utils::make_genesis(&signer);
    let round = genesis_qc.certified_block().round();
    safety_rules.initialize(&proof).unwrap();

    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc, &signer);
    safety_rules.sign_proposal(a1.block().block_data()).unwrap();

    let bad_signer = ValidatorSigner::random([0xfu8; 32]);
    let a2 = make_proposal_with_parent(round + 2, &a1, None, &bad_signer);
    let err = safety_rules
        .sign_proposal(a2.block().block_data())
        .unwrap_err();
    assert_eq!(
        err,
        Error::InvalidProposal("Proposal author is not validator signer!".into())
    );
}

fn test_sign_proposal_with_invalid_qc(safety_rules: &Callback) {
    // Test to sign a proposal with an invalid qc inherited from proposal a2, which
    // is signed by a bad_signer.

    let (mut safety_rules, signer) = safety_rules();

    let (proof, genesis_qc) = test_utils::make_genesis(&signer);
    let round = genesis_qc.certified_block().round();
    safety_rules.initialize(&proof).unwrap();

    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc, &signer);
    safety_rules.sign_proposal(a1.block().block_data()).unwrap();

    let bad_signer = ValidatorSigner::random([0xfu8; 32]);
    let a2 = make_proposal_with_parent(round + 2, &a1, Some(&a1), &bad_signer);
    let a3 =
        test_utils::make_proposal_with_qc(round + 3, a2.block().quorum_cert().clone(), &signer);
    assert_eq!(
        safety_rules
            .sign_proposal(a3.block().block_data())
            .unwrap_err(),
        Error::InvalidQuorumCertificate("Fail to verify QuorumCert".into())
    );
}

fn test_sign_proposal_with_early_preferred_round(safety_rules: &Callback) {
    let (mut safety_rules, signer) = safety_rules();

    let (proof, genesis_qc) = test_utils::make_genesis(&signer);
    let round = genesis_qc.certified_block().round();
    safety_rules.initialize(&proof).unwrap();

    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc, &signer);
    safety_rules.sign_proposal(a1.block().block_data()).unwrap();

    // Update preferred round with a few legal proposals
    let a2 = make_proposal_with_parent(round + 2, &a1, None, &signer);
    let a3 = make_proposal_with_parent(round + 3, &a2, None, &signer);
    let a4 = make_proposal_with_parent(round + 4, &a3, Some(&a2), &signer);
    safety_rules
        .construct_and_sign_vote_two_chain(&a2, None)
        .unwrap();
    safety_rules
        .construct_and_sign_vote_two_chain(&a3, None)
        .unwrap();
    safety_rules
        .construct_and_sign_vote_two_chain(&a4, None)
        .unwrap();

    let a5 = make_proposal_with_qc_and_proof(
        round + 5,
        test_utils::empty_proof(),
        a1.block().quorum_cert().clone(),
        &signer,
    );
    let err = safety_rules
        .sign_proposal(a5.block().block_data())
        .unwrap_err();
    assert_eq!(err, Error::IncorrectPreferredRound(0, 2));
}

fn test_uninitialized_signer(safety_rules: &Callback) {
    // Testing for an uninitialized Option<ValidatorSigner>

    let (mut safety_rules, signer) = safety_rules();

    let (proof, genesis_qc) = test_utils::make_genesis(&signer);
    let round = genesis_qc.certified_block().round();

    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc, &signer);
    let err = safety_rules
        .construct_and_sign_vote_two_chain(&a1, None)
        .unwrap_err();
    assert_eq!(err, Error::NotInitialized("validator_signer".into()));
    let err = safety_rules
        .sign_proposal(a1.block().block_data())
        .unwrap_err();
    assert_eq!(err, Error::NotInitialized("validator_signer".into()));

    safety_rules.initialize(&proof).unwrap();
    safety_rules
        .construct_and_sign_vote_two_chain(&a1, None)
        .unwrap();
}

fn test_validator_not_in_set(safety_rules: &Callback) {
    // Testing for a validator missing from the validator set
    // It does so by updating the safey rule to an epoch state, which does not contain the
    // current validator and check the consensus state

    let (mut safety_rules, signer) = safety_rules();

    let (mut proof, genesis_qc) = test_utils::make_genesis(&signer);
    let round = genesis_qc.certified_block().round();

    safety_rules.initialize(&proof).unwrap();

    // validator_signer is set during initialization
    let state = safety_rules.consensus_state().unwrap();
    assert_eq!(state.in_validator_set(), true);

    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc, &signer);

    // remove the validator_signer in next epoch
    let mut next_epoch_state = EpochState::empty();
    next_epoch_state.epoch = 1;
    let rand_signer = ValidatorSigner::random([0xfu8; 32]);
    next_epoch_state.verifier =
        ValidatorVerifier::new_single(rand_signer.author(), rand_signer.public_key());
    let a2 = test_utils::make_proposal_with_parent_and_overrides(
        Payload::empty(),
        round + 2,
        &a1,
        Some(&a1),
        &signer,
        Some(1),
        Some(next_epoch_state),
    );
    proof
        .ledger_info_with_sigs
        .push(a2.block().quorum_cert().ledger_info().clone());
    assert!(matches!(
        safety_rules.initialize(&proof),
        Err(Error::ValidatorNotInSet(_))
    ));

    let state = safety_rules.consensus_state().unwrap();
    assert_eq!(state.in_validator_set(), false);
}

// Tests for fetching a missing validator key from persistent storage.
fn test_key_not_in_store(safety_rules: &Callback) {
    let (mut safety_rules, signer) = safety_rules();
    let (mut proof, genesis_qc) = test_utils::make_genesis(&signer);
    let round = genesis_qc.certified_block().round();

    safety_rules.initialize(&proof).unwrap();

    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc, &signer);

    // Update to an epoch where the validator fails to retrive the respective key
    // from persistent storage
    let mut next_epoch_state = EpochState::empty();
    next_epoch_state.epoch = 1;
    let rand_signer = ValidatorSigner::random([0xfu8; 32]);
    next_epoch_state.verifier =
        ValidatorVerifier::new_single(signer.author(), rand_signer.public_key());
    let a2 = test_utils::make_proposal_with_parent_and_overrides(
        Payload::empty(),
        round + 2,
        &a1,
        Some(&a1),
        &signer,
        Some(1),
        Some(next_epoch_state),
    );
    proof
        .ledger_info_with_sigs
        .push(a2.block().quorum_cert().ledger_info().clone());

    // Expected failure due to validator key not being found.
    safety_rules.initialize(&proof).unwrap_err();

    let state = safety_rules.consensus_state().unwrap();
    assert_eq!(state.in_validator_set(), false);
}

fn test_2chain_rules(constructor: &Callback) {
    // One chain round is the highest quorum cert round.
    //
    // build a tree of the following form:
    //             _____    _____   _________
    //            /     \  /     \ /         \
    // genesis---a1  b1  b2  a2  b3  a3---a4  b4 a5---a6
    //         \_____/ \_____/ \_____/ \_________/
    //
    let (mut safety_rules, signer) = constructor();
    let (proof, genesis_qc) = test_utils::make_genesis(&signer);
    let genesis_round = genesis_qc.certified_block().round();
    let round = genesis_round;
    safety_rules.initialize(&proof).unwrap();
    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc.clone(), &signer);
    let b1 = test_utils::make_proposal_with_qc(round + 2, genesis_qc, &signer);
    let b2 = make_proposal_with_parent(round + 3, &a1, None, &signer);
    let a2 = make_proposal_with_parent(round + 4, &b1, None, &signer);
    let b3 = make_proposal_with_parent(round + 5, &b2, None, &signer);
    let b4 = make_proposal_with_parent(round + 6, &b3, None, &signer);
    let a3 = make_proposal_with_parent(round + 6, &a2, None, &signer);
    let a4 = make_proposal_with_parent(round + 7, &a3, None, &signer);
    let a5 = make_proposal_with_parent(round + 8, &a3, None, &signer);
    let a6 = make_proposal_with_parent(round + 9, &a5, None, &signer);

    safety_rules.initialize(&proof).unwrap();

    let mut expect = |p, maybe_tc: Option<TwoChainTimeoutCertificate>, vote, commit| {
        let result = safety_rules.construct_and_sign_vote_two_chain(p, maybe_tc.as_ref());
        let qc = p.block().quorum_cert();
        if vote {
            let vote = result.unwrap();
            let id = if commit {
                qc.certified_block().id()
            } else {
                HashValue::zero()
            };
            assert_eq!(vote.ledger_info().consensus_block_id(), id);
            assert!(
                safety_rules.consensus_state().unwrap().one_chain_round()
                    >= qc.certified_block().round()
            );
        } else {
            result.unwrap_err();
        }
    };
    // block == qc + 1, commit
    expect(&a1, None, true, true);
    // block != qc + 1 && block != tc + 1
    expect(
        &b1,
        Some(make_timeout_cert(3, b1.block().quorum_cert(), &signer)),
        false,
        false,
    );
    // block != qc + 1, no TC
    expect(&b2, None, false, false);
    // block = tc + 1, qc == tc.hqc
    expect(
        &a2,
        Some(make_timeout_cert(3, a2.block().quorum_cert(), &signer)),
        true,
        false,
    );
    // block = tc + 1, qc < tc.hqc
    expect(
        &b3,
        Some(make_timeout_cert(4, a3.block().quorum_cert(), &signer)),
        false,
        false,
    );
    // block != qc + 1, no TC
    expect(&a3, None, false, false);
    // block = qc + 1, with TC, commit
    expect(
        &a4,
        Some(make_timeout_cert(7, a3.block().quorum_cert(), &signer)),
        true,
        true,
    );
    // block = tc + 1, qc > tc.hqc
    expect(
        &a5,
        Some(make_timeout_cert(7, b4.block().quorum_cert(), &signer)),
        true,
        false,
    );
    // block = qc + 1, block != tc + 1 (tc is ignored)
    expect(
        &a6,
        Some(make_timeout_cert(7, b4.block().quorum_cert(), &signer)),
        true,
        true,
    );
}

fn test_2chain_timeout(constructor: &Callback) {
    let (mut safety_rules, signer) = constructor();
    let (proof, genesis_qc) = test_utils::make_genesis(&signer);
    let genesis_round = genesis_qc.certified_block().round();
    let round = genesis_round;
    safety_rules.initialize(&proof).unwrap();
    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc.clone(), &signer);
    let a2 = make_proposal_with_parent(round + 2, &a1, None, &signer);
    let a3 = make_proposal_with_parent(round + 3, &a2, None, &signer);

    safety_rules
        .sign_timeout_with_qc(&TwoChainTimeout::new(1, 1, genesis_qc.clone()), None)
        .unwrap();
    assert_eq!(
        safety_rules
            .sign_timeout_with_qc(&TwoChainTimeout::new(1, 2, genesis_qc.clone()), None)
            .unwrap_err(),
        Error::NotSafeToTimeout(2, 0, 0, 0),
    );

    assert_eq!(
        safety_rules
            .sign_timeout_with_qc(&TwoChainTimeout::new(2, 2, genesis_qc.clone()), None)
            .unwrap_err(),
        Error::IncorrectEpoch(2, 1)
    );
    safety_rules
        .sign_timeout_with_qc(
            &TwoChainTimeout::new(1, 2, genesis_qc.clone()),
            Some(make_timeout_cert(1, &genesis_qc, &signer)).as_ref(),
        )
        .unwrap();
    assert_eq!(
        safety_rules
            .sign_timeout_with_qc(&TwoChainTimeout::new(1, 1, genesis_qc.clone()), None)
            .unwrap_err(),
        Error::IncorrectLastVotedRound(1, 2)
    );
    // update one-chain to 2
    safety_rules
        .construct_and_sign_vote_two_chain(&a3, None)
        .unwrap();
    assert_eq!(
        safety_rules
            .sign_timeout_with_qc(
                &TwoChainTimeout::new(1, 4, a3.block().quorum_cert().clone(),),
                Some(make_timeout_cert(2, &genesis_qc, &signer)).as_ref()
            )
            .unwrap_err(),
        Error::NotSafeToTimeout(4, 2, 2, 2)
    );
    assert_eq!(
        safety_rules
            .sign_timeout_with_qc(
                &TwoChainTimeout::new(1, 4, a2.block().quorum_cert().clone(),),
                Some(make_timeout_cert(3, &genesis_qc, &signer)).as_ref()
            )
            .unwrap_err(),
        Error::NotSafeToTimeout(4, 1, 3, 2)
    );
    assert!(matches!(
        safety_rules
            .sign_timeout_with_qc(
                &TwoChainTimeout::new(1, 1, a3.block().quorum_cert().clone(),),
                Some(make_timeout_cert(2, &genesis_qc, &signer)).as_ref()
            )
            .unwrap_err(),
        Error::InvalidTimeout(_)
    ));
}

/// Test that we can succesfully sign a valid commit vote
fn test_sign_commit_vote(constructor: &Callback) {
    // we construct a chain of proposals
    // genesis -- a1 -- a2 -- a3

    let (mut safety_rules, signer) = constructor();
    let (proof, genesis_qc) = test_utils::make_genesis(&signer);

    let round = genesis_qc.certified_block().round();
    safety_rules.initialize(&proof).unwrap();

    let a1 = test_utils::make_proposal_with_qc(round + 1, genesis_qc, &signer);
    let a2 = make_proposal_with_parent(round + 2, &a1, None, &signer);
    let a3 = make_proposal_with_parent(round + 3, &a2, Some(&a1), &signer);

    // now we try to agree on a1's execution result
    let ledger_info_with_sigs = a3.block().quorum_cert().ledger_info();
    // make sure this is for a1
    assert!(ledger_info_with_sigs
        .ledger_info()
        .commit_info()
        .match_ordered_only(
            &a1.block()
                .gen_block_info(*ACCUMULATOR_PLACEHOLDER_HASH, 0, None,)
        ));

    assert!(safety_rules
        .sign_commit_vote(
            ledger_info_with_sigs.clone(),
            ledger_info_with_sigs.ledger_info().clone()
        )
        .is_ok());

    // check empty ledger info
    assert!(matches!(
        safety_rules
            .sign_commit_vote(
                a2.block().quorum_cert().ledger_info().clone(),
                a3.block().quorum_cert().ledger_info().ledger_info().clone()
            )
            .unwrap_err(),
        Error::InvalidOrderedLedgerInfo(_)
    ));

    // non-dummy blockinfo test
    assert!(matches!(
        safety_rules
            .sign_commit_vote(
                LedgerInfoWithSignatures::new(
                    LedgerInfo::new(
                        a1.block().gen_block_info(
                            *ACCUMULATOR_PLACEHOLDER_HASH,
                            100, // non-dummy value
                            None
                        ),
                        ledger_info_with_sigs.ledger_info().consensus_data_hash()
                    ),
                    AggregateSignature::empty(),
                ),
                ledger_info_with_sigs.ledger_info().clone()
            )
            .unwrap_err(),
        Error::InvalidOrderedLedgerInfo(_)
    ));

    // empty signature test
    assert!(matches!(
        safety_rules
            .sign_commit_vote(
                LedgerInfoWithSignatures::new(
                    ledger_info_with_sigs.ledger_info().clone(),
                    AggregateSignature::empty(),
                ),
                ledger_info_with_sigs.ledger_info().clone()
            )
            .unwrap_err(),
        Error::InvalidQuorumCertificate(_)
    ));

    // inconsistent ledger_info test
    let bad_ledger_info = LedgerInfo::new(
        BlockInfo::random(ledger_info_with_sigs.ledger_info().round()),
        ledger_info_with_sigs.ledger_info().consensus_data_hash(),
    );

    assert!(matches!(
        safety_rules
            .sign_commit_vote(ledger_info_with_sigs.clone(), bad_ledger_info,)
            .unwrap_err(),
        Error::InconsistentExecutionResult(_, _)
    ));
}
