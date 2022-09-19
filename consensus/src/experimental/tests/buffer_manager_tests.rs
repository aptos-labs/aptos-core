// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    experimental::{
        buffer_manager::{
            create_channel, BufferManager, OrderedBlocks, Receiver, ResetAck, ResetRequest, Sender,
        },
        decoupled_execution_utils::prepare_phases_and_buffer_manager,
        execution_phase::ExecutionPhase,
        ordering_state_computer::OrderingStateComputer,
        persisting_phase::PersistingPhase,
        pipeline_phase::PipelinePhase,
        signing_phase::SigningPhase,
        tests::test_utils::prepare_executed_blocks_with_ledger_info,
    },
    metrics_safety_rules::MetricsSafetyRules,
    network::NetworkSender,
    network_interface::{ConsensusMsg, ConsensusNetworkSender},
    round_manager::{UnverifiedEvent, VerifiedEvent},
    test_utils::{
        consensus_runtime, timed_block_on, EmptyStateComputer, MockStorage,
        RandomComputeResultStateComputer,
    },
};
use aptos_crypto::{hash::ACCUMULATOR_PLACEHOLDER_HASH, HashValue};
use aptos_infallible::Mutex;
use aptos_secure_storage::Storage;
use aptos_types::{
    account_address::AccountAddress,
    ledger_info::LedgerInfo,
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
    waypoint::Waypoint,
};
use channel::{aptos_channel, message_queues::QueueStyle};
use consensus_types::{
    block::block_test_utils::certificate_for_genesis, executed_block::ExecutedBlock,
    vote_proposal::VoteProposal,
};
use futures::{channel::oneshot, FutureExt, SinkExt, StreamExt};
use itertools::enumerate;
use network::{
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::network::{Event, NewNetworkSender},
};
use safety_rules::{PersistentSafetyStorage, SafetyRulesManager};
use std::sync::Arc;
use tokio::runtime::Runtime;

pub fn prepare_buffer_manager() -> (
    BufferManager,
    Sender<OrderedBlocks>,
    Sender<ResetRequest>,
    aptos_channel::Sender<AccountAddress, VerifiedEvent>,
    channel::Receiver<Event<ConsensusMsg>>,
    PipelinePhase<ExecutionPhase>,
    PipelinePhase<SigningPhase>,
    PipelinePhase<PersistingPhase>,
    HashValue,
    Vec<ValidatorSigner>,
    Receiver<OrderedBlocks>,
    ValidatorVerifier,
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

    let network_sender = ConsensusNetworkSender::new(
        PeerManagerRequestSender::new(network_reqs_tx),
        ConnectionRequestSender::new(connection_reqs_tx),
    );

    let (self_loop_tx, self_loop_rx) = channel::new_test(1000);
    let network = NetworkSender::new(author, network_sender, self_loop_tx, validators.clone());

    let (msg_tx, msg_rx) =
        aptos_channel::new::<AccountAddress, VerifiedEvent>(QueueStyle::FIFO, channel_size, None);

    let (result_tx, result_rx) = create_channel::<OrderedBlocks>();
    let (reset_tx, _) = create_channel::<ResetRequest>();

    let persisting_proxy = Arc::new(OrderingStateComputer::new(
        result_tx,
        Arc::new(EmptyStateComputer),
        reset_tx,
    ));

    let (block_tx, block_rx) = create_channel::<OrderedBlocks>();
    let (buffer_reset_tx, buffer_reset_rx) = create_channel::<ResetRequest>();

    let mocked_execution_proxy = Arc::new(RandomComputeResultStateComputer::new());
    let hash_val = mocked_execution_proxy.get_root_hash();

    let (
        execution_phase_pipeline,
        signing_phase_pipeline,
        persisting_phase_pipeline,
        buffer_manager,
    ) = prepare_phases_and_buffer_manager(
        author,
        mocked_execution_proxy,
        Arc::new(Mutex::new(safety_rules)),
        network,
        msg_rx,
        persisting_proxy,
        block_rx,
        buffer_reset_rx,
        validators.clone(),
    );

    (
        buffer_manager,
        block_tx,
        buffer_reset_tx,
        msg_tx,       // channel to pass commit messages into the buffer manager
        self_loop_rx, // channel to receive message from the buffer manager itself
        execution_phase_pipeline,
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
    aptos_channel::Sender<AccountAddress, VerifiedEvent>,
    channel::Receiver<Event<ConsensusMsg>>,
    HashValue,
    Runtime,
    Vec<ValidatorSigner>,
    Receiver<OrderedBlocks>,
    ValidatorVerifier,
) {
    let runtime = consensus_runtime();

    let (
        buffer_manager,
        block_tx,
        reset_tx,
        msg_tx,       // channel to pass commit messages into the buffer manager
        self_loop_rx, // channel to receive message from the buffer manager itself
        execution_phase_pipeline,
        signing_phase_pipeline,
        persisting_phase_pipeline,
        hash_val,
        signers,
        result_rx,
        validators,
    ) = prepare_buffer_manager();

    runtime.spawn(execution_phase_pipeline.start());
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
    self_loop_rx: &mut channel::Receiver<Event<ConsensusMsg>>,
    msg_tx: &aptos_channel::Sender<AccountAddress, VerifiedEvent>,
    verifier: &ValidatorVerifier,
) {
    match self_loop_rx.next().await {
        Some(Event::Message(author, msg)) => {
            if matches!(msg, ConsensusMsg::CommitVoteMsg(_)) {
                let event: UnverifiedEvent = msg.into();
                // verify the message and send the message into self loop
                msg_tx.push(author, event.verify(verifier).unwrap()).ok();
            }
        }
        _ => {
            panic!("We are expecting a commit vote message.");
        }
    };
}

async fn assert_results(batches: Vec<Vec<ExecutedBlock>>, result_rx: &mut Receiver<OrderedBlocks>) {
    for (i, batch) in enumerate(batches) {
        let OrderedBlocks { ordered_blocks, .. } = result_rx.next().await.unwrap();
        assert_eq!(
            ordered_blocks.last().unwrap().id(),
            batch.last().unwrap().id(),
            "Inconsistent Block IDs (expected {} got {}) for {}-th block",
            batch.last().unwrap().id(),
            ordered_blocks.last().unwrap().id(),
            i,
        );
    }
}

#[test]
fn buffer_manager_happy_path_test() {
    // happy path
    let (
        mut block_tx,
        _reset_tx,
        msg_tx,
        mut self_loop_rx,
        _hash_val,
        mut runtime,
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

    timed_block_on(&mut runtime, async move {
        for i in 0..num_batches {
            block_tx
                .send(OrderedBlocks {
                    ordered_blocks: batches[i].clone(),
                    ordered_proof: proofs[i].clone(),
                    callback: Box::new(move |_, _| {}),
                })
                .await
                .ok();
        }

        // commit decision will be sent too, so 3 * 2
        for _ in 0..6 {
            loopback_commit_vote(&mut self_loop_rx, &msg_tx, &verifier).await;
        }

        // make sure the order is correct
        assert_results(batches, &mut result_rx).await;
    });
}

#[test]
fn buffer_manager_sync_test() {
    // happy path
    let (
        mut block_tx,
        mut reset_tx,
        msg_tx,
        mut self_loop_rx,
        _hash_val,
        mut runtime,
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

    timed_block_on(&mut runtime, async move {
        for i in 0..dropped_batches {
            block_tx
                .send(OrderedBlocks {
                    ordered_blocks: batches[i].clone(),
                    ordered_proof: proofs[i].clone(),
                    callback: Box::new(move |_, _| {}),
                })
                .await
                .ok();
        }

        // reset
        let (tx, rx) = oneshot::channel::<ResetAck>();

        reset_tx.send(ResetRequest { tx, stop: false }).await.ok();
        rx.await.ok();

        // start sending back commit vote after reset, to avoid [0..dropped_batches] being sent to result_rx
        tokio::spawn(async move {
            loop {
                loopback_commit_vote(&mut self_loop_rx, &msg_tx, &verifier).await;
            }
        });

        for i in dropped_batches..num_batches {
            block_tx
                .send(OrderedBlocks {
                    ordered_blocks: batches[i].clone(),
                    ordered_proof: proofs[i].clone(),
                    callback: Box::new(move |_, _| {}),
                })
                .await
                .ok();
        }

        // we should only see batches[dropped_batches..num_batches]
        assert_results(batches.drain(dropped_batches..).collect(), &mut result_rx).await;

        assert!(matches!(result_rx.next().now_or_never(), None));
    });
}
