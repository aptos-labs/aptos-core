// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    consensus_observer::publisher::consensus_publisher::ConsensusPublisher,
    network::{IncomingCommitRequest, NetworkSender},
    pipeline::{
        buffer_manager::{
            create_channel, BufferManager, CriticalErrorNotification, OrderedBlocks, ResetRequest,
        },
        execution_schedule_phase::{ExecutionRequest, ExecutionSchedulePhase},
        execution_wait_phase::{ExecutionResponse, ExecutionWaitPhase, ExecutionWaitRequest},
        persisting_phase::{PersistingPhase, PersistingRequest},
        pipeline_phase::{CountedRequest, PipelinePhase},
        signing_phase::{CommitSignerProvider, SigningPhase, SigningRequest, SigningResponse},
    },
    state_replication::StateComputer,
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::aptos_channel::Receiver;
use aptos_config::config::ConsensusObserverConfig;
use aptos_consensus_types::common::Author;
use aptos_types::{account_address::AccountAddress, epoch_state::EpochState};
use futures::channel::mpsc::UnboundedReceiver;
use std::sync::{
    atomic::{AtomicBool, AtomicU64},
    Arc,
};

/// build channels and return phases and buffer manager
#[allow(clippy::too_many_arguments)]
pub fn prepare_phases_and_buffer_manager(
    author: Author,
    execution_proxy: Arc<dyn StateComputer>,
    safety_rules: Arc<dyn CommitSignerProvider>,
    commit_msg_tx: NetworkSender,
    commit_msg_rx: Receiver<AccountAddress, (AccountAddress, IncomingCommitRequest)>,
    persisting_proxy: Arc<dyn StateComputer>,
    block_rx: UnboundedReceiver<OrderedBlocks>,
    sync_rx: UnboundedReceiver<ResetRequest>,
    epoch_state: Arc<EpochState>,
    bounded_executor: BoundedExecutor,
    order_vote_enabled: bool,
    back_pressure_enabled: bool,
    highest_committed_round: u64,
    consensus_observer_config: ConsensusObserverConfig,
    consensus_publisher: Option<Arc<ConsensusPublisher>>,
    max_pending_rounds_in_commit_vote_cache: u64,
    new_pipeline_enabled: bool,
) -> (
    PipelinePhase<ExecutionSchedulePhase>,
    PipelinePhase<ExecutionWaitPhase>,
    PipelinePhase<SigningPhase>,
    PipelinePhase<PersistingPhase>,
    (BufferManager, UnboundedReceiver<CriticalErrorNotification>),
) {
    let reset_flag = Arc::new(AtomicBool::new(false));
    let ongoing_tasks = Arc::new(AtomicU64::new(0));

    // Execution Phase
    let (execution_schedule_phase_request_tx, execution_schedule_phase_request_rx) =
        create_channel::<CountedRequest<ExecutionRequest>>();
    let (execution_schedule_phase_response_tx, execution_schedule_phase_response_rx) =
        create_channel::<ExecutionWaitRequest>();
    let execution_schedule_phase_processor = ExecutionSchedulePhase::new(execution_proxy);
    let execution_schedule_phase = PipelinePhase::new(
        execution_schedule_phase_request_rx,
        Some(execution_schedule_phase_response_tx),
        Box::new(execution_schedule_phase_processor),
        reset_flag.clone(),
    );

    let (execution_wait_phase_request_tx, execution_wait_phase_request_rx) =
        create_channel::<CountedRequest<ExecutionWaitRequest>>();
    let (execution_wait_phase_response_tx, execution_wait_phase_response_rx) =
        create_channel::<ExecutionResponse>();
    let execution_wait_phase_processor = ExecutionWaitPhase;
    let execution_wait_phase = PipelinePhase::new(
        execution_wait_phase_request_rx,
        Some(execution_wait_phase_response_tx),
        Box::new(execution_wait_phase_processor),
        reset_flag.clone(),
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
        reset_flag.clone(),
    );

    // Persisting Phase
    let (persisting_phase_request_tx, persisting_phase_request_rx) =
        create_channel::<CountedRequest<PersistingRequest>>();
    let (persisting_phase_response_tx, persisting_phase_response_rx) = create_channel();
    let commit_msg_tx = Arc::new(commit_msg_tx);

    let persisting_phase_processor = PersistingPhase::new(persisting_proxy, commit_msg_tx.clone());
    let persisting_phase = PipelinePhase::new(
        persisting_phase_request_rx,
        Some(persisting_phase_response_tx),
        Box::new(persisting_phase_processor),
        reset_flag.clone(),
    );

    // Create the buffer manager
    let buffer_manager_and_error_listener = BufferManager::new(
        author,
        execution_schedule_phase_request_tx,
        execution_schedule_phase_response_rx,
        execution_wait_phase_request_tx,
        execution_wait_phase_response_rx,
        signing_phase_request_tx,
        signing_phase_response_rx,
        commit_msg_tx,
        commit_msg_rx,
        persisting_phase_request_tx,
        persisting_phase_response_rx,
        block_rx,
        sync_rx,
        epoch_state,
        ongoing_tasks,
        reset_flag.clone(),
        bounded_executor,
        order_vote_enabled,
        back_pressure_enabled,
        highest_committed_round,
        consensus_observer_config,
        consensus_publisher,
        max_pending_rounds_in_commit_vote_cache,
        new_pipeline_enabled,
    );

    (
        execution_schedule_phase,
        execution_wait_phase,
        signing_phase,
        persisting_phase,
        buffer_manager_and_error_listener,
    )
}
