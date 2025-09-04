// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pipeline::{
        buffer_manager::{create_channel, Receiver, Sender},
        pipeline_phase::{CountedRequest, PipelinePhase},
        signing_phase::{SigningPhase, SigningRequest, SigningResponse},
        tests::{
            phase_tester::PhaseTester,
            test_utils::{
                prepare_executed_blocks_with_executed_ledger_info,
                prepare_executed_blocks_with_ordered_ledger_info, prepare_safety_rules,
            },
        },
    },
    test_utils::consensus_runtime,
};
use velor_crypto::HashValue;
use velor_safety_rules::Error;
use velor_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_signer::ValidatorSigner,
};
use std::sync::{atomic::AtomicBool, Arc};

pub fn prepare_signing_pipeline(
    signing_phase: SigningPhase,
) -> (
    Sender<CountedRequest<SigningRequest>>,
    Receiver<SigningResponse>,
    PipelinePhase<SigningPhase>,
) {
    // e2e tests
    let (in_channel_tx, in_channel_rx) = create_channel::<CountedRequest<SigningRequest>>();
    let (out_channel_tx, out_channel_rx) = create_channel::<SigningResponse>();
    let reset_flag = Arc::new(AtomicBool::new(false));

    let signing_phase_pipeline = PipelinePhase::new(
        in_channel_rx,
        Some(out_channel_tx),
        Box::new(signing_phase),
        reset_flag,
    );

    (in_channel_tx, out_channel_rx, signing_phase_pipeline)
}

fn add_signing_phase_test_cases(
    phase_tester: &mut PhaseTester<SigningPhase>,
    signers: &[ValidatorSigner],
) {
    let (vecblocks, ordered_ledger_info) =
        prepare_executed_blocks_with_ordered_ledger_info(&signers[0]);
    let commit_ledger_info = LedgerInfo::new(
        vecblocks.last().unwrap().block_info(),
        HashValue::from_u64(0xBEEF),
    );

    // happy path
    phase_tester.add_test_case(
        SigningRequest {
            ordered_ledger_info: ordered_ledger_info.clone(),
            commit_ledger_info: commit_ledger_info.clone(),
            blocks: vecblocks.clone(),
        },
        Box::new(move |resp| {
            assert!(resp.signature_result.is_ok());
            assert_eq!(resp.commit_ledger_info, commit_ledger_info);
        }),
    );

    let (_, executed_ledger_info) = prepare_executed_blocks_with_executed_ledger_info(&signers[0]);
    let executed_commit_ledger_info = executed_ledger_info.ledger_info().clone();
    let inconsistent_commit_ledger_info =
        LedgerInfo::new(BlockInfo::random(1), HashValue::from_u64(0xBEEF));

    // inconsistent
    phase_tester.add_test_case(
        SigningRequest {
            ordered_ledger_info: ordered_ledger_info.clone(),
            commit_ledger_info: inconsistent_commit_ledger_info,
            blocks: vecblocks.clone(),
        },
        Box::new(move |resp| {
            assert!(matches!(
                resp.signature_result,
                Err(Error::InconsistentExecutionResult(_, _))
            ));
        }),
    );

    // ordered ledger info same as commit ledger info
    phase_tester.add_test_case(
        SigningRequest {
            ordered_ledger_info: executed_ledger_info.clone(),
            commit_ledger_info: executed_ledger_info.ledger_info().clone(),
            blocks: vecblocks.clone(),
        },
        Box::new(move |resp| {
            assert!(resp.signature_result.is_ok());
            assert_eq!(resp.commit_ledger_info, executed_commit_ledger_info);
        }),
    );

    // invalid quorum
    phase_tester.add_test_case(
        SigningRequest {
            ordered_ledger_info: LedgerInfoWithSignatures::new(
                ordered_ledger_info.ledger_info().clone(),
                AggregateSignature::empty(),
            ),
            commit_ledger_info: executed_ledger_info.ledger_info().clone(),
            blocks: vecblocks.clone(),
        },
        Box::new(move |resp| {
            assert!(matches!(
                resp.signature_result,
                Err(Error::InvalidQuorumCertificate(_))
            ));
        }),
    );
}

#[test]
fn signing_phase_tests() {
    let runtime = consensus_runtime();

    let (safety_rule_handle, signers) = prepare_safety_rules();

    let signing_phase = SigningPhase::new(safety_rule_handle);

    // unit tests
    let mut unit_phase_tester = PhaseTester::<SigningPhase>::new();
    add_signing_phase_test_cases(&mut unit_phase_tester, &signers);
    unit_phase_tester.unit_test(&signing_phase);

    let (in_channel_tx, out_channel_rx, signing_phase_pipeline) =
        prepare_signing_pipeline(signing_phase);

    runtime.spawn(signing_phase_pipeline.start());

    let mut e2e_phase_tester = PhaseTester::<SigningPhase>::new();
    add_signing_phase_test_cases(&mut e2e_phase_tester, &signers);
    e2e_phase_tester.e2e_test(in_channel_tx, out_channel_rx);
}
