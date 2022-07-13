// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    experimental::{
        buffer_manager::{create_channel, BufferManager, OrderedBlocks, ResetRequest},
        execution_phase::{ExecutionPhase, ExecutionRequest, ExecutionResponse},
        persisting_phase::{PersistingPhase, PersistingRequest},
        pipeline_phase::{CountedRequest, PipelinePhase},
        signing_phase::{SigningPhase, SigningRequest, SigningResponse},
    },
    metrics_safety_rules::MetricsSafetyRules,
    network::NetworkSender,
    round_manager::VerifiedEvent,
    state_replication::StateComputer,
};
use aptos_infallible::Mutex;
use aptos_types::{account_address::AccountAddress, validator_verifier::ValidatorVerifier};
use channel::aptos_channel::Receiver;
use consensus_types::common::Author;
use futures::channel::mpsc::UnboundedReceiver;
use std::sync::{atomic::AtomicU64, Arc};

/// build channels and return phases and buffer manager
pub fn prepare_phases_and_buffer_manager(
    author: Author,
    execution_proxy: Arc<dyn StateComputer>,
    safety_rules: Arc<Mutex<MetricsSafetyRules>>,
    commit_msg_tx: NetworkSender,
    commit_msg_rx: Receiver<AccountAddress, VerifiedEvent>,
    persisting_proxy: Arc<dyn StateComputer>,
    block_rx: UnboundedReceiver<OrderedBlocks>,
    sync_rx: UnboundedReceiver<ResetRequest>,
    verifier: ValidatorVerifier,
) -> (
    PipelinePhase<ExecutionPhase>,
    PipelinePhase<SigningPhase>,
    PipelinePhase<PersistingPhase>,
    BufferManager,
) {
    // Execution Phase
    let (execution_phase_request_tx, execution_phase_request_rx) =
        create_channel::<CountedRequest<ExecutionRequest>>();
    let (execution_phase_response_tx, execution_phase_response_rx) =
        create_channel::<ExecutionResponse>();

    let ongoing_tasks = Arc::new(AtomicU64::new(0));

    let execution_phase_processor = ExecutionPhase::new(execution_proxy);
    let execution_phase = PipelinePhase::new(
        execution_phase_request_rx,
        Some(execution_phase_response_tx),
        Box::new(execution_phase_processor),
    );

    // Signing Phase
    let (signing_phase_request_tx, signing_phase_request_rx) =
        create_channel::<CountedRequest<SigningRequest>>();
    let (signing_phase_response_tx, signing_phase_response_rx) =
        create_channel::<SigningResponse>();

    let signing_phase_processor = SigningPhase::new(safety_rules);
    let signing_phase = PipelinePhase::new(
        signing_phase_request_rx,
        Some(signing_phase_response_tx),
        Box::new(signing_phase_processor),
    );

    // Persisting Phase
    let (persisting_phase_request_tx, persisting_phase_request_rx) =
        create_channel::<CountedRequest<PersistingRequest>>();

    let persisting_phase_processor = PersistingPhase::new(persisting_proxy);
    let persisting_phase = PipelinePhase::new(
        persisting_phase_request_rx,
        None,
        Box::new(persisting_phase_processor),
    );

    (
        execution_phase,
        signing_phase,
        persisting_phase,
        BufferManager::new(
            author,
            execution_phase_request_tx,
            execution_phase_response_rx,
            signing_phase_request_tx,
            signing_phase_response_rx,
            commit_msg_tx,
            commit_msg_rx,
            persisting_phase_request_tx,
            block_rx,
            sync_rx,
            verifier,
            ongoing_tasks,
        ),
    )
}
