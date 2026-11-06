// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    metrics_safety_rules::MetricsSafetyRules,
    network::{IncomingCommitRequest, NetworkSender},
    network_interface::{ConsensusMsg, ConsensusNetworkClient, DIRECT_SEND, RPC},
    pipeline::{
        buffer_manager::{
            create_channel, BufferManager, OrderedBlocks, Receiver, ResetAck, ResetRequest,
            ResetSignal, Sender,
        },
        commit_reliable_broadcast::CommitMessage,
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
use anyhow::anyhow;
use aptos_bounded_executor::BoundedExecutor;
use aptos_channels::{aptos_channel, message_queues::QueueStyle};
use aptos_config::{config::ConsensusObserverConfig, network_id::NetworkId};
use aptos_consensus_types::{
    block::block_test_utils::certificate_for_genesis,
    pipeline::{commit_decision::CommitDecision, commit_vote::CommitVote},
    pipelined_block::{
        CommitLedgerResult, DecryptionOutcome, DecryptionResult, PipelineFutures, PipelineInputTx,
        PipelinedBlock, TaskError, TaskFuture,
    },
    vote_proposal::VoteProposal,
};
use aptos_crypto::{hash::ACCUMULATOR_PLACEHOLDER_HASH, HashValue};
use aptos_executor_types::{
    execution_output::ExecutionOutput, state_compute_result::StateComputeResult,
};
use aptos_infallible::Mutex;
use aptos_network::{
    application::{interface::NetworkClient, storage::PeersAndMetadata},
    peer_manager::{ConnectionRequestSender, PeerManagerRequestSender},
    protocols::{
        network,
        network::{Event, NewNetworkSender},
        wire::handshake::v1::ProtocolId,
    },
};
use aptos_safety_rules::{PersistentSafetyStorage, SafetyRulesManager};
use aptos_secure_storage::Storage;
use aptos_types::{
    account_address::AccountAddress,
    epoch_state::EpochState,
    ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures},
    transaction::CacheInvalidationInfo,
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
    waypoint::Waypoint,
};
use futures::{channel::oneshot, FutureExt, SinkExt, StreamExt};
use itertools::enumerate;
use maplit::hashmap;
use std::{sync::Arc, time::Duration};
use tokio::runtime::Runtime;

pub fn prepare_buffer_manager(
    bounded_executor: BoundedExecutor,
    order_vote_enabled: bool,
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
    let network = Arc::new(NetworkSender::new(
        author,
        consensus_network_client,
        self_loop_tx,
        validators.clone(),
    ));

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
        order_vote_enabled,
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

pub fn launch_buffer_manager(
    order_vote_enabled: bool,
) -> (
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
    ) = prepare_buffer_manager(bounded_executor, order_vote_enabled);

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
        blocks.extend(ordered_blocks);
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
    ) = launch_buffer_manager(false);

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
    ) = launch_buffer_manager(false);

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

fn ready_fut<T: Clone + Send + Sync + 'static>(value: T) -> TaskFuture<T> {
    async move { Ok(value) }.boxed().shared()
}

/// Returns a future that resolves once the returned sender is used.
fn gate() -> (
    tokio::sync::oneshot::Sender<()>,
    TaskFuture<CommitLedgerResult>,
) {
    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let fut = async move {
        rx.await.map_err(|err| TaskError::from(anyhow!(err)))?;
        Ok(None)
    }
    .boxed()
    .shared();
    (tx, fut)
}

/// Sets pipeline futures for a block. Its ledger update resolves after all of
/// `wait_for`, and it commits when the persisting phase sends a commit proof.
/// Returns the ledger info the block commits with.
fn set_fake_pipeline(
    block: &PipelinedBlock,
    compute_result: StateComputeResult,
    wait_for: Vec<TaskFuture<CommitLedgerResult>>,
    signer: &ValidatorSigner,
) -> LedgerInfo {
    // Order votes are enabled, so the commit ledger info has a zero hash.
    let block_info = block.block().gen_block_info(
        compute_result.root_hash(),
        compute_result.last_version_or_0(),
        compute_result.epoch_state().clone(),
    );
    let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
    let commit_vote = CommitVote::new_with_signature(
        signer.author(),
        ledger_info.clone(),
        signer.sign(&ledger_info).unwrap(),
    );

    let ledger_update_result = (compute_result.clone(), Duration::ZERO, None);
    let ledger_update_fut = async move {
        for fut in wait_for {
            fut.await?;
        }
        Ok(ledger_update_result)
    }
    .boxed()
    .shared();

    // Waiting for the commit proof is abortable, as in the real pipeline.
    let (commit_proof_tx, commit_proof_rx) =
        tokio::sync::oneshot::channel::<LedgerInfoWithSignatures>();
    let commit_proof_task = tokio::spawn(commit_proof_rx);
    block.set_pipeline_abort_handles(vec![commit_proof_task.abort_handle()]);
    let block_id = block.id();
    let commit_ledger_fut = async move {
        let li = commit_proof_task
            .await
            .map_err(|err| TaskError::from(anyhow!(err)))?
            .map_err(|err| TaskError::from(anyhow!(err)))?;
        Ok((li.commit_info().id() == block_id).then_some(li))
    }
    .boxed()
    .shared();

    block.set_pipeline_futs(PipelineFutures {
        decryption_fut: ready_fut(DecryptionResult {
            decrypted_txns: vec![],
            retry_txns: vec![],
            regular_txns: vec![],
            max_txns_from_block_to_execute: None,
            block_gas_limit: None,
            outcome: DecryptionOutcome::Disabled,
        }),
        prepare_fut: ready_fut((Arc::new(vec![]), None, DecryptionOutcome::Disabled)),
        has_rand_txns_fut: ready_fut(false),
        rand_check_fut: ready_fut((None, false)),
        execute_fut: ready_fut(Duration::ZERO),
        ledger_update_fut,
        observer_publish_fut: ready_fut(()),
        post_ledger_update_fut: ready_fut(()),
        commit_vote_fut: ready_fut(commit_vote),
        pre_commit_fut: ready_fut(compute_result),
        notify_state_sync_fut: ready_fut(()),
        commit_ledger_fut,
        post_commit_fut: ready_fut(()),
        secret_sharing_derive_self_fut: ready_fut(None),
    });
    block.set_pipeline_tx(PipelineInputTx {
        qc_tx: None,
        rand_tx: None,
        order_vote_tx: None,
        order_proof_tx: None,
        ordered_blocks_for_observer_tx: None,
        commit_proof_tx: Some(commit_proof_tx),
        secret_shared_key_tx: None,
    });
    ledger_info
}

/// Ordered batches on fake pipeline futures, where some blocks require cache
/// invalidation. The child of such a block executes only after it commits.
struct InvalidationTest {
    runtime: Runtime,
    block_tx: Sender<OrderedBlocks>,
    reset_tx: Sender<ResetRequest>,
    msg_tx: aptos_channel::Sender<AccountAddress, (AccountAddress, IncomingCommitRequest)>,
    self_loop_rx: Option<aptos_channels::UnboundedReceiver<Event<ConsensusMsg>>>,
    signer: ValidatorSigner,
    verifier: Arc<ValidatorVerifier>,
    batches: Vec<OrderedBlocks>,
    invalidating: Vec<bool>,
    ledger_infos: Vec<LedgerInfo>,
    commit_futs: Vec<TaskFuture<CommitLedgerResult>>,
}

impl InvalidationTest {
    /// If set, `gates[i]` delays the execution of the i-th block.
    fn new(
        batch_sizes: &[u64],
        invalidating: &[bool],
        gates: Vec<Option<TaskFuture<CommitLedgerResult>>>,
    ) -> Self {
        let (block_tx, reset_tx, msg_tx, self_loop_rx, _, runtime, signers, _, verifier) =
            launch_buffer_manager(true);
        let signer = signers[0].clone();

        // Rounds start at 1, so pending commit votes for them are kept.
        let genesis_qc = certificate_for_genesis();
        let mut init_round = 1;
        let mut last_proposal: Option<VoteProposal> = None;
        let mut batches = vec![];
        for &batch_size in batch_sizes {
            let (ordered_blocks, ordered_proof, proposals) =
                prepare_executed_blocks_with_ledger_info(
                    &signer,
                    batch_size,
                    *ACCUMULATOR_PLACEHOLDER_HASH,
                    *ACCUMULATOR_PLACEHOLDER_HASH,
                    last_proposal,
                    Some(genesis_qc.clone()),
                    init_round,
                );
            init_round += batch_size;
            last_proposal = proposals.last().cloned();
            batches.push(OrderedBlocks {
                ordered_blocks,
                ordered_proof,
            });
        }

        let blocks = batches
            .iter()
            .flat_map(|batch| batch.ordered_blocks.clone())
            .collect::<Vec<_>>();
        assert_eq!(blocks.len(), invalidating.len());
        assert_eq!(blocks.len(), gates.len());

        let _guard = runtime.enter();
        let mut ledger_infos = vec![];
        let mut commit_futs: Vec<TaskFuture<CommitLedgerResult>> = vec![];
        for ((idx, block), gate) in blocks.iter().enumerate().zip(gates) {
            let mut compute_result =
                StateComputeResult::new_dummy_with_root_hash(HashValue::random());
            if invalidating[idx] {
                compute_result.execution_output =
                    ExecutionOutput::new_dummy_with_cache_invalidation_info(
                        CacheInvalidationInfo::V0 { published: vec![] },
                    );
            }
            let mut wait_for = gate.into_iter().collect::<Vec<_>>();
            if idx > 0 && invalidating[idx - 1] {
                wait_for.push(commit_futs[idx - 1].clone());
            }
            ledger_infos.push(set_fake_pipeline(block, compute_result, wait_for, &signer));
            commit_futs.push(block.pipeline_futs().unwrap().commit_ledger_fut);
        }

        Self {
            runtime,
            block_tx,
            reset_tx,
            msg_tx,
            self_loop_rx: Some(self_loop_rx),
            signer,
            verifier,
            batches,
            invalidating: invalidating.to_vec(),
            ledger_infos,
            commit_futs,
        }
    }

    fn new_ungated(batch_sizes: &[u64], invalidating: &[bool]) -> Self {
        Self::new(batch_sizes, invalidating, vec![None; invalidating.len()])
    }

    /// Loops our own commit votes back into the buffer manager.
    fn loop_back_commit_votes(&mut self) {
        let mut self_loop_rx = self.self_loop_rx.take().unwrap();
        let msg_tx = self.msg_tx.clone();
        let verifier = self.verifier.clone();
        self.runtime.spawn(async move {
            while let Some(msg) = self_loop_rx.next().await {
                loopback_commit_vote(msg, &msg_tx, &verifier).await;
            }
        });
    }

    fn send_batches(&self) {
        for batch in &self.batches {
            self.block_tx.unbounded_send(batch.clone()).unwrap();
        }
    }

    /// Sends a commit message, and waits until the buffer manager acks it.
    async fn send_commit_message(&self, req: CommitMessage) {
        let author = self.signer.author();
        let protocol = ProtocolId::ConsensusRpcBcs;
        let (response_sender, response_rx) = oneshot::channel();
        self.msg_tx
            .push(
                author,
                (author, IncomingCommitRequest {
                    req,
                    protocol,
                    response_sender,
                }),
            )
            .unwrap();
        let response = protocol
            .from_bytes::<ConsensusMsg>(&response_rx.await.unwrap().unwrap())
            .unwrap();
        assert!(matches!(
            response,
            ConsensusMsg::CommitMessage(msg) if matches!(*msg, CommitMessage::Ack(()))
        ));
    }

    fn commit_vote(&self, idx: usize) -> CommitMessage {
        let ledger_info = self.ledger_infos[idx].clone();
        let signature = self.signer.sign(&ledger_info).unwrap();
        CommitMessage::Vote(CommitVote::new_with_signature(
            self.signer.author(),
            ledger_info,
            signature,
        ))
    }

    fn commit_decision(&self, idx: usize) -> CommitMessage {
        CommitMessage::Decision(CommitDecision::new(generate_ledger_info_with_sig(
            std::slice::from_ref(&self.signer),
            self.ledger_infos[idx].clone(),
        )))
    }

    /// Checks that every block that requires invalidation, and the last
    /// block, commits with its own ledger info.
    async fn assert_commits(&self) {
        let tips = self
            .batches
            .iter()
            .map(|batch| batch.ordered_blocks.last().unwrap().id())
            .collect::<Vec<_>>();
        let blocks = self.batches.iter().flat_map(|batch| &batch.ordered_blocks);
        for (idx, block) in blocks.enumerate() {
            let committed = self.commit_futs[idx].clone().await.unwrap();
            let committed_id = committed.map(|li| li.commit_info().id());
            if self.invalidating[idx] || idx + 1 == self.commit_futs.len() {
                assert_eq!(committed_id, Some(block.id()), "block {}", idx);
            } else if tips.contains(&block.id()) {
                // A batch tip can also commit as a prefix of a later batch.
                assert!(
                    committed_id.is_none_or(|id| id == block.id()),
                    "block {}",
                    idx
                );
            } else {
                assert_eq!(committed_id, None, "block {}", idx);
            }
        }
    }
}

fn run_with_commit_vote_loopback(batch_sizes: &[u64], invalidating: &[bool]) {
    let mut test = InvalidationTest::new_ungated(batch_sizes, invalidating);
    test.loop_back_commit_votes();
    timed_block_on(&test.runtime, async {
        test.send_batches();
        test.assert_commits().await;
    });
}

#[test]
fn buffer_manager_cut_after_invalidating_block() {
    run_with_commit_vote_loopback(&[2], &[true, false]);
}

#[test]
fn buffer_manager_cut_before_invalidating_tip() {
    // The next batch waits for the remaining tip's commit.
    run_with_commit_vote_loopback(&[2, 1], &[true, true, false]);
}

#[test]
fn buffer_manager_cut_after_consecutive_invalidating_blocks() {
    run_with_commit_vote_loopback(&[3, 1], &[true, true, true, false]);
}

#[test]
fn buffer_manager_no_cut_without_invalidation() {
    run_with_commit_vote_loopback(&[3, 2], &[false, false, false, false, false]);
}

#[test]
fn buffer_manager_cut_uses_pending_commit_votes() {
    // Without the loopback, blocks commit only with the votes sent here.
    let test = InvalidationTest::new_ungated(&[2], &[true, false]);
    timed_block_on(&test.runtime, async {
        // Both votes are pending, because no item has these blocks as its tip.
        test.send_commit_message(test.commit_vote(0)).await;
        test.send_commit_message(test.commit_vote(1)).await;
        test.send_batches();
        test.assert_commits().await;
    });
}

#[test]
fn buffer_manager_cut_keeps_commit_proof_of_tip() {
    let (open_gate, gate) = gate();
    let test = InvalidationTest::new(&[2], &[true, false], vec![Some(gate), None]);
    timed_block_on(&test.runtime, async {
        // The first block is not executed yet, so its decision is pending, and
        // the tip's decision is stored in the ordered item.
        test.send_batches();
        test.send_commit_message(test.commit_decision(1)).await;
        test.send_commit_message(test.commit_decision(0)).await;
        open_gate.send(()).unwrap();
        test.assert_commits().await;
    });
}

#[test]
fn buffer_manager_reset_while_waiting_for_invalidating_block() {
    let mut test = InvalidationTest::new_ungated(&[2], &[true, false]);
    let mut self_loop_rx = test.self_loop_rx.take().unwrap();
    timed_block_on(&test.runtime, async {
        test.send_batches();

        // The first block's own commit vote means it was cut. Without the
        // loopback it never commits, so the tip keeps waiting.
        self_loop_rx.next().await.unwrap();
        let (tx, rx) = oneshot::channel::<ResetAck>();
        test.reset_tx
            .unbounded_send(ResetRequest {
                tx,
                signal: ResetSignal::TargetRound(0),
            })
            .unwrap();
        rx.await.unwrap();

        for commit_fut in &test.commit_futs {
            assert!(commit_fut.clone().await.is_err());
        }
    });
}
