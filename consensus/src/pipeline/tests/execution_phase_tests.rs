// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pipeline::{
        buffer_manager::create_channel,
        execution_schedule_phase::{ExecutionRequest, ExecutionSchedulePhase},
        execution_wait_phase::{ExecutionResponse, ExecutionWaitPhase},
        pipeline_phase::{CountedRequest, PipelinePhase, StatelessPipeline},
        tests::phase_tester::PhaseTester,
    },
    state_replication::StateComputer,
    test_utils::{consensus_runtime, RandomComputeResultStateComputer},
};
use aptos_consensus_types::{
    block::{block_test_utils::certificate_for_genesis, Block},
    common::Payload,
    pipelined_block::PipelinedBlock,
    quorum_cert::QuorumCert,
};
use aptos_crypto::HashValue;
use aptos_executor_types::{state_compute_result::StateComputeResult, ExecutorError};
use aptos_types::{ledger_info::LedgerInfo, validator_verifier::random_validator_verifier};
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    Arc,
};

// ExecutionSchedulePhase and ExecutionWaitPhase chained together.
// In BufferManager they are chained through the main loop.
pub struct ExecutionPhaseForTest {
    schedule_phase: ExecutionSchedulePhase,
    wait_phase: ExecutionWaitPhase,
}

impl ExecutionPhaseForTest {
    pub fn new(execution_proxy: Arc<dyn StateComputer>) -> Self {
        let schedule_phase = ExecutionSchedulePhase::new(execution_proxy);
        let wait_phase = ExecutionWaitPhase;
        Self {
            schedule_phase,
            wait_phase,
        }
    }
}

#[async_trait]
impl StatelessPipeline for ExecutionPhaseForTest {
    type Request = ExecutionRequest;
    type Response = ExecutionResponse;

    const NAME: &'static str = "execution";

    async fn process(&self, req: ExecutionRequest) -> ExecutionResponse {
        let wait_req = self.schedule_phase.process(req).await;
        self.wait_phase.process(wait_req).await
    }
}

pub fn prepare_execution_phase() -> (HashValue, ExecutionPhaseForTest) {
    let execution_proxy = Arc::new(RandomComputeResultStateComputer::new());
    let random_hash_value = execution_proxy.get_root_hash();
    let execution_phase = ExecutionPhaseForTest::new(execution_proxy);

    (random_hash_value, execution_phase)
}

fn dummy_guard() -> CountedRequest<()> {
    CountedRequest::new((), Arc::new(AtomicU64::new(0)))
}

fn add_execution_phase_test_cases(
    phase_tester: &mut PhaseTester<ExecutionPhaseForTest>,
    random_hash_value: HashValue,
) {
    let genesis_qc = certificate_for_genesis();
    let (signers, _validators) = random_validator_verifier(1, None, false);
    let block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        genesis_qc,
        &signers[0],
        Vec::new(),
    )
    .unwrap();

    // happy path
    phase_tester.add_test_case(
        ExecutionRequest {
            ordered_blocks: vec![Arc::new(PipelinedBlock::new(
                block,
                vec![],
                StateComputeResult::new_dummy(),
            ))],
            lifetime_guard: dummy_guard(),
        },
        Box::new(move |resp| {
            assert_eq!(
                resp.inner.unwrap()[0].compute_result().root_hash(),
                random_hash_value
            );
        }),
    );

    // empty block
    phase_tester.add_test_case(
        ExecutionRequest {
            ordered_blocks: vec![],
            lifetime_guard: dummy_guard(),
        },
        Box::new(move |resp| assert!(matches!(resp.inner, Err(ExecutorError::EmptyBlocks)))),
    );

    // bad parent id
    let bad_qc = QuorumCert::certificate_for_genesis_from_ledger_info(
        &LedgerInfo::mock_genesis(None),
        random_hash_value,
    );
    let bad_block = Block::new_proposal(
        Payload::empty(false, true),
        1,
        1,
        bad_qc,
        &signers[0],
        Vec::new(),
    )
    .unwrap();
    phase_tester.add_test_case(
        ExecutionRequest {
            ordered_blocks: vec![Arc::new(PipelinedBlock::new(
                bad_block,
                vec![],
                StateComputeResult::new_dummy(),
            ))],
            lifetime_guard: dummy_guard(),
        },
        Box::new(move |resp| assert!(matches!(resp.inner, Err(ExecutorError::BlockNotFound(_))))),
    );
}

#[test]
fn execution_phase_tests() {
    let runtime = consensus_runtime();

    // unit tests
    let (random_hash_value, execution_phase) = prepare_execution_phase();
    let mut unit_phase_tester = PhaseTester::<ExecutionPhaseForTest>::new();
    add_execution_phase_test_cases(&mut unit_phase_tester, random_hash_value);
    unit_phase_tester.unit_test(&execution_phase);

    // e2e tests
    let (in_channel_tx, in_channel_rx) = create_channel::<CountedRequest<ExecutionRequest>>();
    let (out_channel_tx, out_channel_rx) = create_channel::<ExecutionResponse>();
    let reset_flag = Arc::new(AtomicBool::new(false));

    let execution_phase_pipeline = PipelinePhase::new(
        in_channel_rx,
        Some(out_channel_tx),
        Box::new(execution_phase),
        reset_flag,
    );

    runtime.spawn(execution_phase_pipeline.start());

    let mut e2e_phase_tester = PhaseTester::<ExecutionPhaseForTest>::new();
    add_execution_phase_test_cases(&mut e2e_phase_tester, random_hash_value);
    e2e_phase_tester.e2e_test(in_channel_tx, out_channel_rx);
}
