// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Consensus commit stages and their input boundary. Known blocks live only in
//! BlockTree; this runtime owns no block queue or stage-completion scheduler.
use crate::{
    block_storage::{
        block_tree::BlockTree,
        tracing::{observe_block, BlockStage},
    },
    consensus_observer::{
        network::observer_message::ConsensusObserverMessage,
        publisher::consensus_publisher::ConsensusPublisher,
    },
    counters,
    network::{IncomingCommitRequest, NetworkSender},
    pipeline::{
        buffer_manager::{ResetAck, ResetRequest, ResetSignal},
        commit_certificate::CommitCertificate,
        commit_reliable_broadcast::{AckState, CommitMessage},
    },
    IntGaugeGuard,
};
use anyhow::anyhow;
use aptos_bounded_executor::BoundedExecutor;
use aptos_consensus_types::{
    block::Block,
    common::{Author, Round},
    pipelined_block::{
        execution_ledger_info, CommitInput, LedgerUpdateResult, PipelinedBlock, TaskFuture,
        TaskResult,
    },
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_crypto::HashValue;
use aptos_infallible::{Mutex, RwLock};
use aptos_logger::warn;
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_change::EpochChangeProof, epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
};
use futures::{channel::mpsc::UnboundedReceiver, StreamExt};
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Weak},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_retry::strategy::ExponentialBackoff;

struct Inputs {
    tree: Weak<RwLock<BlockTree>>,
    // Only messages whose block has not been inserted. One vote per author per
    // round, bounded relative to the durable root; never a known-block queue.
    votes:
        BTreeMap<Round, HashMap<Author, aptos_consensus_types::pipeline::commit_vote::CommitVote>>,
    proofs: BTreeMap<Round, LedgerInfoWithSignatures>,
    stopped: bool,
    generation: u64,
    floor: Round,
}

pub struct CommitPipelineRuntime {
    inputs: Mutex<Inputs>,
    drain_lock: tokio::sync::Mutex<()>,
    epoch: Arc<EpochState>,
    network: Arc<NetworkSender>,
    publisher: Option<Arc<ConsensusPublisher>>,
    broadcast: ReliableBroadcast<CommitMessage, ExponentialBackoff>,
    max_pending_rounds: u64,
}

impl CommitPipelineRuntime {
    pub fn new(
        epoch: Arc<EpochState>,
        network: Arc<NetworkSender>,
        executor: BoundedExecutor,
        publisher: Option<Arc<ConsensusPublisher>>,
        max_pending_rounds: u64,
        floor: Round,
        author: Author,
    ) -> Arc<Self> {
        Arc::new(Self {
            broadcast: ReliableBroadcast::new(
                "commit_pipeline",
                author,
                epoch.verifier.get_ordered_account_addresses(),
                network.clone(),
                ExponentialBackoff::from_millis(2)
                    .factor(50)
                    .max_delay(Duration::from_secs(5)),
                TimeService::real(),
                Duration::from_millis(1500),
                executor,
            ),
            inputs: Mutex::new(Inputs {
                tree: Weak::new(),
                votes: BTreeMap::new(),
                proofs: BTreeMap::new(),
                stopped: false,
                generation: 0,
                floor,
            }),
            drain_lock: tokio::sync::Mutex::new(()),
            epoch,
            network,
            publisher,
            max_pending_rounds,
        })
    }

    pub(crate) fn bind_tree(&self, tree: &Arc<RwLock<BlockTree>>) {
        let mut inputs = self.inputs.lock();
        inputs.tree = Arc::downgrade(tree);
        inputs.stopped = false;
    }

    /// Called after insertion to deliver inputs that arrived before the block.
    pub fn deliver_cached(&self, block: &PipelinedBlock) {
        let mut inputs = self.inputs.lock();
        if inputs.stopped {
            return;
        }
        let Some(tx) = block
            .pipeline_tx()
            .lock()
            .as_ref()
            .and_then(|tx| tx.commit_messages_tx.clone())
        else {
            return;
        };
        if let Some(votes) = inputs.votes.get_mut(&block.round()) {
            votes.retain(|_, vote| {
                vote.commit_info().id() != block.id()
                    || tx.try_send(CommitInput::Vote(vote.clone())).is_err()
            });
            if votes.is_empty() {
                inputs.votes.remove(&block.round());
            }
        }
        if let Some(proof) = inputs.proofs.get(&block.round()) {
            if proof.commit_info().id() == block.id()
                && tx.try_send(CommitInput::Decision(proof.clone())).is_ok()
            {
                inputs.proofs.remove(&block.round());
            }
        }
    }

    #[cfg(test)]
    pub(super) fn route_for_test(&self, message: CommitMessage) -> bool {
        let generation = self.inputs.lock().generation;
        self.route(message, generation)
    }

    fn route(&self, message: CommitMessage, generation: u64) -> bool {
        let (id, round, input) = match message {
            CommitMessage::Vote(vote) => (
                vote.commit_info().id(),
                vote.commit_info().round(),
                CommitInput::Vote(vote),
            ),
            CommitMessage::Decision(proof) => (
                proof.ledger_info().commit_info().id(),
                proof.ledger_info().commit_info().round(),
                CommitInput::Decision(proof.into_inner()),
            ),
            CommitMessage::Ack(_) | CommitMessage::Nack => return false,
        };
        let mut inputs = self.inputs.lock();
        if inputs.stopped || inputs.generation != generation {
            return false;
        }
        let tree = inputs.tree.upgrade();
        let floor = tree.as_ref().map_or(inputs.floor, |tree| {
            tree.read().commit_root().round().max(inputs.floor)
        });
        inputs.votes.retain(|round, _| *round > floor);
        inputs.proofs.retain(|round, _| *round > floor);
        if round <= floor {
            return true;
        }
        let block = tree.and_then(|tree| tree.read().get_block(&id));
        if let Some(block) = block {
            let info = match &input {
                CommitInput::Vote(v) => v.commit_info(),
                CommitInput::Decision(p) => p.commit_info(),
            };
            if !block.block_info().match_ordered_only(info) {
                return false;
            }
            let tx = block
                .pipeline_tx()
                .lock()
                .as_ref()
                .and_then(|tx| tx.commit_messages_tx.clone());
            return tx.is_some_and(|tx| tx.try_send(input).is_ok());
        }
        if round >= floor.saturating_add(self.max_pending_rounds) {
            return false;
        }
        match input {
            CommitInput::Vote(vote) => {
                inputs
                    .votes
                    .entry(round)
                    .or_default()
                    .insert(vote.author(), vote);
            },
            CommitInput::Decision(proof) => {
                inputs.proofs.insert(round, proof);
            },
        }
        true
    }

    pub async fn certificate(
        &self,
        block: Arc<Block>,
        ledger_update: TaskFuture<LedgerUpdateResult>,
        order_proof: TaskFuture<WrappedLedgerInfo>,
        mut messages: mpsc::Receiver<CommitInput>,
        order_vote_enabled: bool,
    ) -> TaskResult<LedgerInfoWithSignatures> {
        let ordered = order_proof.await?;
        let ordered_guard =
            IntGaugeGuard::new(counters::NUM_BLOCKS_IN_PIPELINE.with_label_values(&["ordered"]));
        let (result, elapsed, timestamp) = ledger_update.await?;
        drop(ordered_guard);
        let pipelined_block = {
            let inputs = self.inputs.lock();
            inputs
                .tree
                .upgrade()
                .and_then(|tree| tree.read().get_block(&block.id()))
        };
        if let Some(pipelined_block) = pipelined_block {
            if pipelined_block.get_execution_summary().is_none() {
                pipelined_block.set_compute_result(result.clone(), elapsed);
            }
        }
        // Intermediate blocks publish execution feedback too, but only the
        // batch target aggregates votes over this ordering proof's hash.
        if ordered.commit_info().id() != block.id() {
            return std::future::pending().await;
        }
        let _executed_guard = IntGaugeGuard::new(
            counters::NUM_BLOCKS_IN_PIPELINE.with_label_values(&["certificate"]),
        );
        let hash = if order_vote_enabled {
            HashValue::zero()
        } else {
            ordered.ledger_info().ledger_info().consensus_data_hash()
        };
        let mut certificate = CommitCertificate::new(ordered.commit_info().clone());
        certificate.set_execution_result(execution_ledger_info(&block, &result, timestamp, hash));
        while let Some(message) = messages.recv().await {
            match message {
                CommitInput::Vote(vote) => {
                    certificate.add_vote(vote);
                },
                CommitInput::Decision(proof) => certificate.add_decision(proof),
            }
            if let Some(proof) = certificate.certificate(&self.epoch.verifier) {
                return Ok(proof);
            }
        }
        Err(anyhow!("Commit input closed").into())
    }

    /// A descendant certificate is an input to every uncommitted ancestor's
    /// storage future. The block tree provides identity/path, never stage state.
    pub async fn certify_ancestors(
        &self,
        certificate: TaskFuture<LedgerInfoWithSignatures>,
    ) -> TaskResult<()> {
        let proof = certificate.await?;
        let blocks = {
            let inputs = self.inputs.lock();
            if inputs.stopped {
                return Err(anyhow!("Pipeline reset").into());
            }
            let tree = inputs
                .tree
                .upgrade()
                .ok_or_else(|| anyhow!("Block tree dropped"))?;
            let path = tree.read().path_from_commit_root(proof.commit_info().id());
            path.ok_or_else(|| anyhow!("Certified block is not on the committed path"))?
        };
        // Capture and finish execution before delivering the proof, without any
        // tree lock held. Parent storage ordering remains in commit_ledger_fut.
        let mut executed = Vec::with_capacity(blocks.len());
        for block in blocks {
            let futures = block
                .pipeline_futs()
                .ok_or_else(|| anyhow!("Pipeline aborted"))?;
            let (result, elapsed, _) = futures.ledger_update_fut.await?;
            if block.get_execution_summary().is_none() {
                block.set_compute_result(result, elapsed);
            }
            executed.push(block);
        }
        let mut delivered = false;
        {
            let mut inputs = self.inputs.lock();
            if inputs.stopped {
                return Err(anyhow!("Pipeline reset").into());
            }
            for block in executed {
                if let Some(tx) = block
                    .pipeline_tx()
                    .lock()
                    .as_mut()
                    .and_then(|tx| tx.commit_proof_tx.take())
                {
                    let store = aptos_transaction_tracing::store::TransactionTraceStore::global();
                    if store.is_enabled() {
                        store.record_block_stage(
                            &block.id(),
                            aptos_transaction_tracing::types::TransactionStage::Certified,
                        );
                    }
                    let _ = tx.send(proof.clone());
                    delivered = true;
                }
            }
            if delivered && proof.ledger_info().ends_epoch() {
                // Seal proof admission atomically with the first epoch-ending
                // delivery. Concurrent descendants must remain an uncertified
                // suffix while that prefix finishes durable storage.
                inputs.stopped = true;
                inputs.generation += 1;
            }
        }
        if !delivered {
            return Ok(());
        }
        observe_block(
            proof.commit_info().timestamp_usecs(),
            BlockStage::COMMIT_CERTIFIED,
        );
        if let Some(publisher) = &self.publisher {
            publisher.publish_message(ConsensusObserverMessage::new_commit_decision_message(proof));
        }
        Ok(())
    }

    pub async fn broadcast_vote(
        &self,
        vote: TaskFuture<aptos_consensus_types::pipeline::commit_vote::CommitVote>,
        proof: TaskFuture<LedgerInfoWithSignatures>,
        order: TaskFuture<WrappedLedgerInfo>,
    ) -> TaskResult<()> {
        let vote = tokio::select! { vote = vote => vote?, _ = proof.clone() => return Ok(()) };
        let ordered =
            tokio::select! { ordered = order => ordered?, _ = proof.clone() => return Ok(()) };
        // Preserve early one-shot vote publication in commit_vote_fut, while
        // reliable retries belong only to ordered batch targets.
        if ordered.commit_info().id() != vote.commit_info().id() {
            return Ok(());
        }
        let _signed_guard =
            IntGaugeGuard::new(counters::NUM_BLOCKS_IN_PIPELINE.with_label_values(&["signed"]));
        let vote = commit_message(vote);
        // Restart reliable broadcast after all peers ack but no certificate has
        // arrived. Its retry work is owned and cancelled by this future.
        loop {
            let broadcast = self.broadcast.broadcast(
                vote.clone(),
                AckState::new(self.epoch.verifier.get_ordered_account_addresses_iter()),
            );
            tokio::select! { _ = proof.clone() => return Ok(()), _ = broadcast => {} }
            tokio::select! { _ = proof.clone() => return Ok(()), _ = tokio::time::sleep(Duration::from_secs(30)) => {} }
        }
    }

    pub async fn epoch_change(
        &self,
        commit: TaskFuture<Option<LedgerInfoWithSignatures>>,
        post_commit: TaskFuture<()>,
    ) -> TaskResult<()> {
        let proof = commit.await?;
        post_commit.await?;
        if let Some(proof) = proof.filter(|proof| proof.ledger_info().ends_epoch()) {
            self.drain(true).await;
            self.network
                .send_epoch_change(EpochChangeProof::new(vec![proof], false))
                .await;
        }
        Ok(())
    }

    pub async fn drain(&self, stop: bool) {
        let _drain = self.drain_lock.lock().await;
        let blocks = {
            let mut inputs = self.inputs.lock();
            inputs.stopped = true;
            inputs.generation += 1;
            inputs.votes.clear();
            inputs.proofs.clear();
            inputs
                .tree
                .upgrade()
                .map(|tree| tree.read().get_all_blocks())
                .unwrap_or_default()
        };
        let mut certified = vec![];
        let mut suffix = vec![];
        for block in blocks {
            let has_proof = block
                .pipeline_tx()
                .lock()
                .as_ref()
                .is_some_and(|tx| tx.commit_proof_tx.is_none());
            if has_proof {
                if let Some(futures) = block.pipeline_futs() {
                    certified.push(futures);
                }
            } else {
                suffix.push(block);
            }
        }
        for futures in certified {
            let _ = futures.commit_ledger_fut.clone().await;
            let _ = futures.post_commit_fut.clone().await;
            futures.wait_until_finishes().await;
        }
        // Abort the entire suffix before awaiting non-abortable storage tasks.
        let aborted: Vec<_> = suffix
            .into_iter()
            .filter_map(|block| block.abort_pipeline())
            .collect();
        for futures in aborted {
            futures.wait_until_finishes().await;
        }
        if !stop {
            self.inputs.lock().stopped = false;
        }
    }

    /// Input-only network boundary: verify and route, or consume reset requests.
    pub async fn run(
        self: Arc<Self>,
        mut messages: aptos_channels::aptos_channel::Receiver<
            Author,
            (Author, IncomingCommitRequest),
        >,
        mut resets: UnboundedReceiver<ResetRequest>,
        executor: BoundedExecutor,
    ) {
        let (verified_tx, mut verified_rx) = futures::channel::mpsc::unbounded();
        let runtime = self.clone();
        let verification = tokio::spawn(async move {
            while let Some((author, request)) = messages.next().await {
                let tx = verified_tx.clone();
                let runtime = runtime.clone();
                let generation = runtime.inputs.lock().generation;
                executor
                    .spawn_blocking(move || {
                        match request.req.verify(author, &runtime.epoch.verifier) {
                            Ok(()) => {
                                let _ = tx.unbounded_send((generation, request));
                            },
                            Err(error) => warn!("Invalid commit message: {}", error),
                        }
                    })
                    .await;
            }
        });
        loop {
            tokio::select! {
                Some((generation, request)) = verified_rx.next() => {
                    let accepted = self.route(request.req, generation);
                    reply(request.protocol, request.response_sender, accepted);
                },
                Some(request) = resets.next() => {
                    let stop = matches!(request.signal, ResetSignal::Stop);
                    self.drain(stop).await;
                    if let ResetSignal::TargetRound(round) = request.signal { self.inputs.lock().floor = round; }
                    let _ = request.tx.send(ResetAck::default());
                    if stop { break; }
                },
                else => break,
            }
        }
        verification.abort();
        let _ = verification.await;
    }
}

fn commit_message(vote: aptos_consensus_types::pipeline::commit_vote::CommitVote) -> CommitMessage {
    fail::fail_point!("consensus::create_invalid_commit_vote", |_| {
        CommitMessage::Vote(
            aptos_consensus_types::pipeline::commit_vote::CommitVote::new_with_signature(
                vote.author(),
                vote.ledger_info().clone(),
                aptos_crypto::bls12381::Signature::dummy_signature(),
            ),
        )
    });
    CommitMessage::Vote(vote)
}

fn reply(
    protocol: aptos_network::protocols::wire::handshake::v1::ProtocolId,
    sender: futures::channel::oneshot::Sender<
        Result<bytes::Bytes, aptos_network::protocols::rpc::error::RpcError>,
    >,
    accepted: bool,
) {
    let response = crate::network_interface::ConsensusMsg::CommitMessage(Box::new(
        if accepted {
            CommitMessage::Ack(())
        } else {
            CommitMessage::Nack
        },
    ));
    if let Ok(bytes) = protocol.to_bytes(&response) {
        let _ = sender.send(Ok(bytes.into()));
    }
}
