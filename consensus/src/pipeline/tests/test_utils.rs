// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{metrics_safety_rules::MetricsSafetyRules, test_utils::MockStorage};
use velor_consensus_types::{
    block::block_test_utils::certificate_for_genesis,
    common::{Payload, Round},
    pipelined_block::PipelinedBlock,
    quorum_cert::QuorumCert,
    vote_proposal::VoteProposal,
};
use velor_crypto::{hash::ACCUMULATOR_PLACEHOLDER_HASH, HashValue};
use velor_executor_types::state_compute_result::StateComputeResult;
use velor_infallible::Mutex;
use velor_safety_rules::{
    test_utils::{make_proposal_with_parent, make_proposal_with_qc},
    PersistentSafetyStorage, SafetyRulesManager,
};
use velor_secure_storage::Storage;
use velor_types::{
    ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures},
    validator_signer::ValidatorSigner,
    validator_verifier::random_validator_verifier,
    waypoint::Waypoint,
};
use std::sync::Arc;

pub fn prepare_safety_rules() -> (Arc<Mutex<MetricsSafetyRules>>, Vec<ValidatorSigner>) {
    let num_nodes = 1;

    // environment setup
    let (signers, validators) = random_validator_verifier(num_nodes, None, false);
    let validator_set = (&validators).into();
    let signer = &signers[0];

    let waypoint =
        Waypoint::new_epoch_boundary(&LedgerInfo::mock_genesis(Some(validator_set))).unwrap();

    let safety_storage = PersistentSafetyStorage::initialize(
        Storage::from(velor_secure_storage::InMemoryStorage::new()),
        signer.author(),
        signer.private_key().clone(),
        waypoint,
        true,
    );
    let (_, storage) = MockStorage::start_for_testing((&validators).into());

    let safety_rules_manager = SafetyRulesManager::new_serializer(safety_storage);
    let mut safety_rules = MetricsSafetyRules::new(safety_rules_manager.client(), storage);
    safety_rules.perform_initialize().unwrap();

    (Arc::new(Mutex::new(safety_rules)), signers)
}

// This function prioritizes using parent over init_qc
pub fn prepare_executed_blocks_with_ledger_info(
    signer: &ValidatorSigner,
    num_blocks: Round,
    executed_hash: HashValue,
    consensus_hash: HashValue,
    some_parent: Option<VoteProposal>,
    init_qc: Option<QuorumCert>,
    init_round: Round,
) -> (
    Vec<Arc<PipelinedBlock>>,
    LedgerInfoWithSignatures,
    Vec<VoteProposal>,
) {
    assert!(num_blocks > 0);

    let p1 = if let Some(parent) = some_parent {
        make_proposal_with_parent(
            Payload::empty(false, true),
            init_round,
            &parent,
            None,
            signer,
        )
    } else {
        make_proposal_with_qc(init_round, init_qc.unwrap(), signer)
    };

    let mut proposals: Vec<VoteProposal> = vec![p1];

    for i in 1..num_blocks {
        println!("Generating {}", i);
        let parent = proposals.last().unwrap();
        let proposal = make_proposal_with_parent(
            Payload::empty(false, true),
            init_round + i,
            parent,
            None,
            signer,
        );
        proposals.push(proposal);
    }

    let compute_result = StateComputeResult::new_dummy_with_root_hash(executed_hash);

    let li = LedgerInfo::new(
        proposals.last().unwrap().block().gen_block_info(
            compute_result.root_hash(),
            compute_result.last_version_or_0(),
            compute_result.epoch_state().clone(),
        ),
        consensus_hash,
    );

    let li_sig = generate_ledger_info_with_sig(&[signer.clone()], li);

    let executed_blocks: Vec<Arc<PipelinedBlock>> = proposals
        .iter()
        .map(|proposal| {
            Arc::new(PipelinedBlock::new(
                proposal.block().clone(),
                vec![],
                compute_result.clone(),
            ))
        })
        .collect();

    (executed_blocks, li_sig, proposals)
}

pub fn prepare_executed_blocks_with_executed_ledger_info(
    signer: &ValidatorSigner,
) -> (Vec<Arc<PipelinedBlock>>, LedgerInfoWithSignatures) {
    let genesis_qc = certificate_for_genesis();
    let (executed_blocks, li_sig, _) = prepare_executed_blocks_with_ledger_info(
        signer,
        1,
        HashValue::random(),
        HashValue::from_u64(0xBEEF),
        None,
        Some(genesis_qc),
        0,
    );
    (executed_blocks, li_sig)
}

pub fn prepare_executed_blocks_with_ordered_ledger_info(
    signer: &ValidatorSigner,
) -> (Vec<Arc<PipelinedBlock>>, LedgerInfoWithSignatures) {
    let genesis_qc = certificate_for_genesis();
    let (executed_blocks, li_sig, _) = prepare_executed_blocks_with_ledger_info(
        signer,
        1,
        *ACCUMULATOR_PLACEHOLDER_HASH,
        *ACCUMULATOR_PLACEHOLDER_HASH,
        None,
        Some(genesis_qc),
        0,
    );
    (executed_blocks, li_sig)
}
