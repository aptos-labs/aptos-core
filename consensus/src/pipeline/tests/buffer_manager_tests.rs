// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics_safety_rules::MetricsSafetyRules,
    network::{IncomingCommitRequest, NetworkSender},
    network_interface::{ConsensusMsg, ConsensusNetworkClient, DIRECT_SEND, RPC},
    pipeline::{
        buffer_manager::{
            create_channel, BufferManager, OrderedBlocks, Receiver, ResetAck, ResetRequest,
            ResetSignal, Sender,
        },
        decoupled_execution_utils::prepare_phases_and_buffer_manager,
        execution_schedule_phase::ExecutionSchedulePhase,
        execution_wait_phase::ExecutionWaitPhase,
        persisting_phase::PersistingPhase,
        pipeline_phase::PipelinePhase,
        signing_phase::SigningPhase,
        tests::test_utils::prepare_executed_blocks_with_ledger_info,
    },
    test_utils::{
        consensus_runtime, timed_block_on, MockStorage, RandomComputeResultStateComputer,
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{config::ConsensusObserverConfig, network_id::NetworkId};
use aptos_consensus_types::{
    block::block_test_utils::certificate_for_genesis, pipelined_block::PipelinedBlock,
    vote_proposal::VoteProposal,
};
use aptos_crypto::{hash::ACCUMULATOR_PLACEHOLDER_HASH, HashValue};
use aptos_infallible::Mutex;
use aptos_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{
        network,
        network::{Event, NewNetworkSender},
    },
};
use aptos_safety_rules::{PersistentSafetyStorage, SafetyRulesManager};
use aptos_secure_storage::Storage;
use aptos_types::{
    account_address::AccountAddress,
    epoch_state::EpochState,
    ledger_info::LedgerInfo,
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
    waypoint::Waypoint,
};
use futures::{channel::oneshot, FutureExt, SinkExt, StreamExt};
use itertools::enumerate;
use maplit::hashmap;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub fn prepare_buffer_manager(
    bounded_executor: BoundedExecutor,
) -> (
    BufferManager,
    Sender<OrderedBlocks>,
    Sender<ResetRequest>,
    aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingCommitRequest)>,
    aptos_channels::UnboundedReceiver<Event<ConsensusMsg>>,
    PipelinePhase<ExecutionSchedulePhase>,
    PipelinePhase<ExecutionWaitPhase>,
    PipelinePhase<SigningPhase>,
    PipelinePhase<PersistingPhase>,
    HashValue,
    Vec<ValidatorSigner>,
    Receiver<OrderedBlocks>,
    Arc<ValidatorVerifier>,
) {
    let num_nodes = 1;
    let channel_size = 30;

    let (signers, validators) = random_validator_verifier(num_nodes, None, false);
    let signer = &signers[0];
    let author = signer.author();
    let validator_set = (&validators).into();

    let waypoint =
        Waypoint::new_epoch_boundary(&LedgerInfo::mock_genesis(Some(validator_set))).unwrap();

    let safety_storage = PersistentSafetyStorage::initialize(
        Storage::from(aptos_secure_storage::InMemoryStorage::new()),
        signer.author(),
        signer.private_key().clone(),
        waypoint,
        true,
    );
    let (_, storage) = MockStorage::start_for_testing((&validators).into());

    let safety_rules_manager = SafetyRulesManager::new_local(safety_storage);

    let mut safety_rules = MetricsSafetyRules::new(safety_rules_manager.client(), storage);
    safety_rules.perform_initialize().unwrap();

    let (network_reqs_tx, _network_reqs_rx) = aptos_channel::new(QueueStyle::FIFO, 8, None);
    let (connection_reqs_tx, _) = aptos_channel::new(QueueStyle::FIFO, 8, None);
    let network_sender = network::NetworkSender::new(
        PeerManagerRequestSender::new(network_reqs_tx),
        ConnectionRequestSender::new(connection_reqs_tx),
    );
    let network_client = NetworkClient::new(
        DIRECT_SEND.into(),
        RPC.into(),
        hashmap! {NetworkId::Validator => network_sender},
        PeersAndMetadata::new(&[NetworkId::Validator]),
    );
    let consensus_network_client = ConsensusNetworkClient::new(network_client);

    let (self_loop_tx, self_loop_rx) = aptos_channels::new_unbounded_test();
    let validators = Arc::new(validators);
    let network = NetworkSender::new(
        author,
        consensus_network_client,
        self_loop_tx,
        validators.clone(),
    );

    let (msg_tx, msg_rx) = aptos_channel::new::<
        AccountAddress,
        (AccountAddress, IncomingCommitRequest),
    >(QueueStyle::FIFO, channel_size, None);

    let (_result_tx, result_rx) = create_channel::<OrderedBlocks>();

    let (block_tx, block_rx) = create_channel::<OrderedBlocks>();
    let (buffer_reset_tx, buffer_reset_rx) = create_channel::<ResetRequest>();

    let mocked_execution_proxy = Arc::new(RandomComputeResultStateComputer::new());
    let hash_val = mocked_execution_proxy.get_root_hash();

    let (
        execution_schedule_phase_pipeline,
        execution_wait_phase_pipeline,
        signing_phase_pipeline,
        persisting_phase_pipeline,
        buffer_manager,
    ) = prepare_phases_and_buffer_manager(
        author,
        Arc::new(Mutex::new(safety_rules)),
        network,
        msg_rx,
        block_rx,
        buffer_reset_rx,
        Arc::new(EpochState {
            epoch: 1,
            verifier: validators.clone(),
        }),
        bounded_executor,
        false,
        true,
        0,
        ConsensusObserverConfig::default(),
        None,
        100,
    );

    (
        buffer_manager,
        block_tx,
        buffer_reset_tx,
        msg_tx,       // channel to pass commit messages into the buffer manager
        self_loop_rx, // channel to receive message from the buffer manager itself
        execution_schedule_phase_pipeline,
        execution_wait_phase_pipeline,
        signing_phase_pipeline,
        persisting_phase_pipeline,
        hash_val,
        signers,
        result_rx,
        validators,
    )
}

pub fn launch_buffer_manager() -> (
    Sender<OrderedBlocks>,
    Sender<ResetRequest>,
    aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingCommitRequest)>,
    aptos_channels::UnboundedReceiver<Event<ConsensusMsg>>,
    HashValue,
    Runtime,
    Vec<ValidatorSigner>,
    Receiver<OrderedBlocks>,
    Arc<ValidatorVerifier>,
) {
    let runtime = consensus_runtime();

    let bounded_executor: BoundedExecutor = BoundedExecutor::new(1, runtime.handle().clone());
    let (
        buffer_manager,
        block_tx,
        reset_tx,
        msg_tx,       // channel to pass commit messages into the buffer manager
        self_loop_rx, // channel to receive message from the buffer manager itself
        execution_schedule_phase_pipeline,
        execution_wait_phase_pipeline,
        signing_phase_pipeline,
        persisting_phase_pipeline,
        hash_val,
        signers,
        result_rx,
        validators,
    ) = prepare_buffer_manager(bounded_executor);

    runtime.spawn(execution_schedule_phase_pipeline.start());
    runtime.spawn(execution_wait_phase_pipeline.start());
    runtime.spawn(signing_phase_pipeline.start());
    runtime.spawn(persisting_phase_pipeline.start());
    runtime.spawn(buffer_manager.start());

    (
        block_tx,
        reset_tx,
        msg_tx,
        self_loop_rx,
        hash_val,
        runtime,
        signers,
        result_rx,
        validators,
    )
}

async fn loopback_commit_vote(
    msg: Event<ConsensusMsg>,
    msg_tx: &aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingCommitRequest)>,
    verifier: &ValidatorVerifier,
) {
    match msg {
        Event::RpcRequest(author, msg, protocol, callback) => {
            if let ConsensusMsg::CommitMessage(msg) = msg {
                msg.verify(author, verifier).unwrap();
                let request = IncomingCommitRequest {
                    req: *msg,
                    protocol,
                    response_sender: callback,
                };
                // verify the message and send the message into self loop
                msg_tx.push(author, (author, request)).ok();
            }
        },
        _ => {
            panic!("We are expecting a commit vote message.");
        },
    };
}

async fn assert_results(
    batches: Vec<Vec<Arc<PipelinedBlock>>>,
    result_rx: &mut Receiver<OrderedBlocks>,
) {
    let total_batches = batches.iter().flatten().count();
    let mut blocks: Vec<Arc<PipelinedBlock>> = Vec::new();
    while blocks.len() < total_batches {
        let OrderedBlocks { ordered_blocks, .. } = result_rx.next().await.unwrap();
        blocks.extend(ordered_blocks.into_iter());
    }

    for (i, batch) in enumerate(batches) {
        for (idx, ordered_block) in blocks.drain(..batch.len()).enumerate() {
            assert_eq!(
                ordered_block.id(),
                batch[idx].id(),
                "Inconsistent Block IDs (expected {} got {}) for {}-th block",
                batch[idx].id(),
                ordered_block.id(),
                i,
            );
        }
    }
}

#[test]
#[ignore]
fn buffer_manager_happy_path_test() {
    // happy path
    let (
        mut block_tx,
        _reset_tx,
        msg_tx,
        mut self_loop_rx,
        _hash_val,
        runtime,
        signers,
        mut result_rx,
        verifier,
    ) = launch_buffer_manager();

    let genesis_qc = certificate_for_genesis();
    let num_batches = 3;
    let blocks_per_batch = 5;
    let mut init_round = 0;

    let mut batches = vec![];
    let mut proofs = vec![];
    let mut last_proposal: Option<VoteProposal> = None;

    for _ in 0..num_batches {
        let (vecblocks, li_sig, proposal) = prepare_executed_blocks_with_ledger_info(
            &signers[0],
            blocks_per_batch,
            *ACCUMULATOR_PLACEHOLDER_HASH,
            *ACCUMULATOR_PLACEHOLDER_HASH,
            last_proposal,
            Some(genesis_qc.clone()),
            init_round,
        );
        init_round += blocks_per_batch;
        batches.push(vecblocks);
        proofs.push(li_sig);
        last_proposal = Some(proposal.last().unwrap().clone());
    }

    timed_block_on(&runtime, async move {
        for i in 0..num_batches {
            block_tx
                .send(OrderedBlocks {
                    ordered_blocks: batches[i].clone(),
                    ordered_proof: proofs[i].clone(),
                })
                .await
                .ok();
        }

        // Only commit votes are sent, so 3 commit votes are expected
        // Commit decision is no longer broadcasted
        for _ in 0..3 {
            if let Some(msg) = self_loop_rx.next().await {
                loopback_commit_vote(msg, &msg_tx, &verifier).await;
            }
        }

        // make sure the order is correct
        assert_results(batches, &mut result_rx).await;
    });
}

#[test]
#[ignore]
fn buffer_manager_sync_test() {
    // happy path
    let (
        mut block_tx,
        mut reset_tx,
        msg_tx,
        mut self_loop_rx,
        _hash_val,
        runtime,
        signers,
        mut result_rx,
        verifier,
    ) = launch_buffer_manager();

    let genesis_qc = certificate_for_genesis();
    let num_batches = 100;
    let blocks_per_batch = 5;
    let mut init_round = 0;

    let mut batches = vec![];
    let mut proofs = vec![];
    let mut last_proposal: Option<VoteProposal> = None;

    for _ in 0..num_batches {
        let (vecblocks, li_sig, proposal) = prepare_executed_blocks_with_ledger_info(
            &signers[0],
            blocks_per_batch,
            *ACCUMULATOR_PLACEHOLDER_HASH,
            *ACCUMULATOR_PLACEHOLDER_HASH,
            last_proposal,
            Some(genesis_qc.clone()),
            init_round,
        );
        init_round += blocks_per_batch;
        batches.push(vecblocks);
        proofs.push(li_sig);
        last_proposal = Some(proposal.last().unwrap().clone());
    }

    let dropped_batches = 42;

    timed_block_on(&runtime, async move {
        for i in 0..dropped_batches {
            block_tx
                .send(OrderedBlocks {
                    ordered_blocks: batches[i].clone(),
                    ordered_proof: proofs[i].clone(),
                })
                .await
                .ok();
        }

        // reset
        let (tx, rx) = oneshot::channel::<ResetAck>();

        reset_tx
            .send(ResetRequest {
                tx,
                signal: ResetSignal::TargetRound(1),
            })
            .await
            .ok();
        rx.await.ok();

        // start sending back commit vote after reset, to avoid [0..dropped_batches] being sent to result_rx
        tokio::spawn(async move {
            while let Some(msg) = self_loop_rx.next().await {
                loopback_commit_vote(msg, &msg_tx, &verifier).await;
            }
        });

        for i in dropped_batches..num_batches {
            block_tx
                .send(OrderedBlocks {
                    ordered_blocks: batches[i].clone(),
                    ordered_proof: proofs[i].clone(),
                })
                .await
                .ok();
        }

        // we should only see batches[dropped_batches..num_batches]
        assert_results(batches.drain(dropped_batches..).collect(), &mut result_rx).await;

        assert!(result_rx.next().now_or_never().is_none());
    });
}
