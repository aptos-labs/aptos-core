// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0
// Parts of the project are originally copyright © Meta Platforms, Inc.

use crate::{
    network_tests::NetworkPlayground,
    round_manager::round_manager_tests::NodeSetup,
    test_utils::{consensus_runtime, create_vec_signed_transactions, timed_block_on},
};
use velor_config::config::ConsensusConfig;
use velor_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    common::Payload,
};
use velor_types::{
    dkg::{real_dkg::RealDKG, DKGSessionMetadata, DKGTrait, DKGTranscript},
    jwks::QuorumCertifiedUpdate,
    on_chain_config::{
        ConsensusAlgorithmConfig, ConsensusConfigV1, OnChainConsensusConfig,
        OnChainJWKConsensusConfig, OnChainRandomnessConfig, RandomnessConfigMoveStruct,
        ValidatorTxnConfig, DEFAULT_WINDOW_SIZE,
    },
    validator_signer::ValidatorSigner,
    validator_txn::ValidatorTransaction,
    validator_verifier::{
        random_validator_verifier_with_voting_power, ValidatorConsensusInfoMoveStruct,
        ValidatorVerifier,
    },
};
use rand::{rngs::ThreadRng, thread_rng};

#[test]
/// If ProposalExt feature is disabled, ProposalExt should be rejected
/// No votes are sent, but the block is still added to the block tree.
fn no_vote_on_proposal_ext_when_feature_disabled() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    // In order to observe the votes we're going to check proposal processing on the non-proposer
    // node (which will send the votes to the proposer).
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        None,
        None,
        None,
        None,
        None,
        false,
    );
    let node = &mut nodes[0];
    let genesis_qc = certificate_for_genesis();

    let invalid_block = Block::new_proposal_ext(
        vec![ValidatorTransaction::dummy(vec![0xFF]); 5],
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    let valid_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc,
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    timed_block_on(&runtime, async {
        // clear the message queue
        node.next_proposal().await;

        assert!(node
            .round_manager
            .process_proposal(invalid_block)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal(valid_block)
            .await
            .is_ok());
    });
}

#[test]
fn no_vote_on_proposal_with_unexpected_vtxns() {
    let vtxns = vec![ValidatorTransaction::ObservedJWKUpdate(
        QuorumCertifiedUpdate::dummy(),
    )];

    assert_process_proposal_result(
        None,
        None,
        Some(OnChainJWKConsensusConfig::default_disabled()),
        vtxns.clone(),
        false,
    );

    assert_process_proposal_result(
        None,
        None,
        Some(OnChainJWKConsensusConfig::default_enabled()),
        vtxns,
        true,
    );
}

#[test]
fn no_vote_on_proposal_with_uncertified_dkg_result() {
    test_dkg_result_handling(
        &[25_000_000; 4],
        1,
        RealDKG::sample_secret_and_generate_transcript,
        false,
    );
}

#[test]
fn no_vote_on_proposal_with_inconsistent_secret_dkg_result() {
    test_dkg_result_handling(
        &[10_000_000, 70_000_000, 10_000_000, 10_000_000],
        1,
        RealDKG::generate_transcript_for_inconsistent_secrets,
        false,
    );
}

#[test]
fn no_vote_on_proposal_with_dup_dealers_in_dkg_transcript() {
    test_dkg_result_handling(
        &[10_000_000, 40_000_000, 10_000_000, 40_000_000],
        1,
        RealDKG::deal_twice_and_aggregate,
        false,
    );
}

#[test]
fn vote_on_proposal_with_valid_dkg_result() {
    test_dkg_result_handling(
        &[10_000_000, 70_000_000, 10_000_000, 10_000_000],
        1,
        RealDKG::sample_secret_and_generate_transcript,
        true,
    );
}

fn test_dkg_result_handling<F>(
    voting_powers: &[u64],
    dealer_idx: usize,
    trx_gen_func: F,
    should_accept: bool,
) where
    F: Fn(
        &mut ThreadRng,
        &<RealDKG as DKGTrait>::PublicParams,
        u64,
        &<RealDKG as DKGTrait>::DealerPrivateKey,
    ) -> <RealDKG as DKGTrait>::Transcript,
{
    let mut rng = thread_rng();
    let epoch = 123;
    let num_validators = voting_powers.len();
    let (signers, verifier) =
        random_validator_verifier_with_voting_power(num_validators, None, false, voting_powers);
    let validator_set: Vec<ValidatorConsensusInfoMoveStruct> = verifier
        .validator_infos
        .clone()
        .into_iter()
        .map(ValidatorConsensusInfoMoveStruct::from)
        .collect();

    let dkg_session_metadata = DKGSessionMetadata {
        dealer_epoch: epoch,
        randomness_config: RandomnessConfigMoveStruct::from(
            OnChainRandomnessConfig::default_enabled(),
        ),
        dealer_validator_set: validator_set.clone(),
        target_validator_set: validator_set,
    };
    let public_params = RealDKG::new_public_params(&dkg_session_metadata);
    let trx = trx_gen_func(
        &mut rng,
        &public_params,
        dealer_idx as u64,
        signers[dealer_idx].private_key(),
    );
    let trx_bytes = bcs::to_bytes(&trx).unwrap();
    let vtxns = vec![ValidatorTransaction::DKGResult(DKGTranscript::new(
        epoch,
        verifier.get_ordered_account_addresses()[dealer_idx],
        trx_bytes,
    ))];

    assert_process_proposal_result(
        Some((signers, verifier)),
        Some(OnChainRandomnessConfig::default_enabled()),
        Some(OnChainJWKConsensusConfig::default_enabled()),
        vtxns.clone(),
        should_accept,
    );
}

/// Setup a node with default configs and an optional `Features` override.
/// Create a block, fill it with the given vtxns, and process it with the `RoundManager` from the setup.
/// Assert the processing result.
fn assert_process_proposal_result(
    validator_set: Option<(Vec<ValidatorSigner>, ValidatorVerifier)>,
    randomness_config: Option<OnChainRandomnessConfig>,
    jwk_consensus_config: Option<OnChainJWKConsensusConfig>,
    vtxns: Vec<ValidatorTransaction>,
    expected_result: bool,
) {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());
    let mut nodes = NodeSetup::create_nodes_with_validator_set(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        Some(OnChainConsensusConfig::default_for_genesis()),
        None,
        None,
        randomness_config,
        jwk_consensus_config,
        validator_set,
        false,
    );

    let node = &mut nodes[0];
    let genesis_qc = certificate_for_genesis();
    let block = Block::new_proposal_ext(
        vtxns,
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    timed_block_on(&runtime, async {
        // clear the message queue
        node.next_proposal().await;

        assert_eq!(
            expected_result,
            node.round_manager
                .process_proposal(block.clone())
                .await
                .is_ok()
        );
    });
}

#[ignore]
#[test]
/// If receiving txn num/block size limit is exceeded, ProposalExt should be rejected.
/// TODO: re-implement dummy vtxn and re-enable.
fn no_vote_on_proposal_ext_when_receiving_limit_exceeded() {
    let runtime = consensus_runtime();
    let mut playground = NetworkPlayground::new(runtime.handle().clone());

    let alg_config = ConsensusAlgorithmConfig::JolteonV2 {
        main: ConsensusConfigV1::default(),
        quorum_store_enabled: true,
        order_vote_enabled: false,
    };
    let vtxn_config = ValidatorTxnConfig::V1 {
        per_block_limit_txn_count: 5,
        per_block_limit_total_bytes: 400,
    };

    let local_config = ConsensusConfig {
        max_receiving_block_txns: 10,
        max_receiving_block_bytes: 800,
        ..Default::default()
    };

    let randomness_config = OnChainRandomnessConfig::default_enabled();
    let mut nodes = NodeSetup::create_nodes(
        &mut playground,
        runtime.handle().clone(),
        1,
        None,
        Some(OnChainConsensusConfig::V4 {
            alg: alg_config,
            vtxn: vtxn_config,
            window_size: DEFAULT_WINDOW_SIZE,
        }),
        None,
        Some(local_config),
        Some(randomness_config),
        None,
        false,
    );
    let node = &mut nodes[0];
    let genesis_qc = certificate_for_genesis();

    let block_too_many_txns = Block::new_proposal_ext(
        vec![],
        Payload::DirectMempool(create_vec_signed_transactions(11)),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    let block_too_many_vtxns = Block::new_proposal_ext(
        vec![ValidatorTransaction::dummy(vec![0xFF; 20]); 6],
        Payload::DirectMempool(create_vec_signed_transactions(4)),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    let block_too_large = Block::new_proposal_ext(
        vec![ValidatorTransaction::dummy(vec![0xFF; 200]); 1], // total_bytes >= 200 * 1 = 200
        Payload::DirectMempool(create_vec_signed_transactions(9)), // = total_bytes >= 69 * 9 = 621
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    let block_vtxns_too_large = Block::new_proposal_ext(
        vec![ValidatorTransaction::dummy(vec![0xFF; 200]); 5], // total_bytes >= 200 * 5 = 1000
        Payload::empty(false, true),
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    let valid_block = Block::new_proposal_ext(
        vec![ValidatorTransaction::dummy(vec![0xFF; 20]); 5], // total_bytes >= 60 * 5 = 300
        Payload::DirectMempool(create_vec_signed_transactions(5)), // total_bytes >= 69 * 5 = 345
        1,
        1,
        genesis_qc.clone(),
        &node.signer,
        Vec::new(),
    )
    .unwrap();

    timed_block_on(&runtime, async {
        // clear the message queue
        node.next_proposal().await;

        assert!(node
            .round_manager
            .process_proposal(block_too_many_txns)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal(block_too_many_vtxns)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal(block_too_large)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal(block_vtxns_too_large)
            .await
            .is_err());

        assert!(node
            .round_manager
            .process_proposal(valid_block)
            .await
            .is_ok());
    });
}
