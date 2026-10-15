// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::tracing::{observe_block, BlockStage},
    consensus_observer::{
        network::observer_message::ConsensusObserverMessage,
        publisher::consensus_publisher::ConsensusPublisher,
    },
    counters,
    network::{IncomingCommitRequest, NetworkSender},
    network_interface::ConsensusMsg,
    pipeline::{
        commit_certificate::CommitCertificate,
        commit_reliable_broadcast::{AckState, CommitMessage},
    },
};
use aptos_bounded_executor::BoundedExecutor;
use aptos_config::config::ConsensusObserverConfig;
use aptos_consensus_types::{
    common::{Author, Round},
    pipeline::commit_vote::CommitVote,
    pipelined_block::{execution_ledger_info, PipelinedBlock, TaskResult},
};
use aptos_crypto::HashValue;
use aptos_logger::prelude::*;
use aptos_network::protocols::wire::handshake::v1::ProtocolId;
use aptos_reliable_broadcast::{DropGuard, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{
    account_address::AccountAddress,
    epoch_change::EpochChangeProof,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
};
use bytes::Bytes;
use futures::{
    channel::{
        mpsc::{unbounded, UnboundedReceiver},
        oneshot,
    },
    future::{AbortHandle, Abortable, BoxFuture},
    stream::FuturesUnordered,
    FutureExt, StreamExt,
};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};
use tokio::time::{Duration, Instant};
use tokio_retry::strategy::ExponentialBackoff;

const BROADCAST_INTERVAL_MS: u64 = 1500;
const REBROADCAST_INTERVAL_MS: u64 = 30000;

#[derive(Debug, Default)]
pub struct ResetAck {}
pub enum ResetSignal {
    Stop,
    TargetRound(u64),
}
pub struct ResetRequest {
    pub tx: oneshot::Sender<ResetAck>,
    pub signal: ResetSignal,
}
#[derive(Clone)]
pub struct OrderedBlocks {
    pub ordered_blocks: Vec<Arc<PipelinedBlock>>,
    pub ordered_proof: LedgerInfoWithSignatures,
}

struct Entry {
    blocks: Vec<Arc<PipelinedBlock>>,
    certificate: CommitCertificate,
    local_vote: Option<CommitVote>,
    broadcast: Option<(Instant, DropGuard)>,
}

enum Completion {
    Executed(HashValue, TaskResult<LedgerInfo>),
    Voted(HashValue, TaskResult<CommitVote>),
}

/// Auxiliary certificate inbox and ordered-prefix index. Execution, signing and
/// persistence are driven exclusively by the block's pipeline futures.
pub struct CommitPipeline {
    entries: HashMap<HashValue, Entry>,
    prefix: VecDeque<HashValue>,
    pending_commit_blocks: BTreeMap<Round, Arc<PipelinedBlock>>,
    pending_votes: BTreeMap<Round, HashMap<Author, CommitVote>>,
    pending_proofs: BTreeMap<Round, LedgerInfoWithSignatures>,
    completions: FuturesUnordered<BoxFuture<'static, Completion>>,
    commits: FuturesUnordered<BoxFuture<'static, (Round, TaskResult<()>)>>,
    highest_committed_round: Round,
    max_pending_rounds: u64,
    order_vote_enabled: bool,
    observer_enabled: bool,
    epoch_state: Arc<EpochState>,
    network: Arc<NetworkSender>,
    publisher: Option<Arc<ConsensusPublisher>>,
    reliable_broadcast: ReliableBroadcast<CommitMessage, ExponentialBackoff>,
    bounded_executor: BoundedExecutor,
    block_rx: UnboundedReceiver<OrderedBlocks>,
    reset_rx: UnboundedReceiver<ResetRequest>,
    commit_rx: Option<
        aptos_channels::aptos_channel::Receiver<
            AccountAddress,
            (AccountAddress, IncomingCommitRequest),
        >,
    >,
    generation: Arc<AtomicU64>,
    previous_commit_time: Instant,
}

impl CommitPipeline {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        author: Author,
        network: Arc<NetworkSender>,
        commit_rx: aptos_channels::aptos_channel::Receiver<
            AccountAddress,
            (AccountAddress, IncomingCommitRequest),
        >,
        block_rx: UnboundedReceiver<OrderedBlocks>,
        reset_rx: UnboundedReceiver<ResetRequest>,
        epoch_state: Arc<EpochState>,
        bounded_executor: BoundedExecutor,
        order_vote_enabled: bool,
        highest_committed_round: Round,
        observer_config: ConsensusObserverConfig,
        publisher: Option<Arc<ConsensusPublisher>>,
        max_pending_rounds: u64,
    ) -> Self {
        let reliable_broadcast = ReliableBroadcast::new(
            "commit_pipeline",
            author,
            epoch_state.verifier.get_ordered_account_addresses(),
            network.clone(),
            ExponentialBackoff::from_millis(2)
                .factor(50)
                .max_delay(Duration::from_secs(5)),
            TimeService::real(),
            Duration::from_millis(BROADCAST_INTERVAL_MS),
            bounded_executor.clone(),
        );
        Self {
            entries: HashMap::new(),
            prefix: VecDeque::new(),
            pending_commit_blocks: BTreeMap::new(),
            pending_votes: BTreeMap::new(),
            pending_proofs: BTreeMap::new(),
            completions: FuturesUnordered::new(),
            commits: FuturesUnordered::new(),
            highest_committed_round,
            max_pending_rounds,
            order_vote_enabled,
            observer_enabled: observer_config.observer_enabled,
            epoch_state,
            network,
            publisher,
            reliable_broadcast,
            bounded_executor,
            block_rx,
            reset_rx,
            commit_rx: Some(commit_rx),
            generation: Arc::new(AtomicU64::new(0)),
            previous_commit_time: Instant::now(),
        }
    }

    fn broadcast(&self, vote: CommitVote) -> Option<(Instant, DropGuard)> {
        if self.observer_enabled {
            return None;
        }
        fail::fail_point!("consensus::create_invalid_commit_vote", |_| {
            let faulty = CommitVote::new_with_signature(
                vote.author(),
                vote.ledger_info().clone(),
                aptos_crypto::bls12381::Signature::dummy_signature(),
            );
            self.broadcast_message(CommitMessage::Vote(faulty))
        });
        self.broadcast_message(CommitMessage::Vote(vote))
    }

    fn broadcast_message(&self, message: CommitMessage) -> Option<(Instant, DropGuard)> {
        let (handle, registration) = AbortHandle::new_pair();
        let task = self.reliable_broadcast.broadcast(
            message,
            AckState::new(
                self.epoch_state
                    .verifier
                    .get_ordered_account_addresses_iter(),
            ),
        );
        tokio::spawn(Abortable::new(task, registration));
        Some((Instant::now(), DropGuard::new(handle)))
    }

    fn register(&mut self, blocks: OrderedBlocks) {
        let Some(last) = blocks.ordered_blocks.last() else {
            return;
        };
        let id = last.id();
        let round = last.round();
        let Some(last_futures) = last.pipeline_futs() else {
            error!("Ordered block {} has no pipeline futures", id);
            return;
        };
        assert!(
            !self.entries.contains_key(&id),
            "Ordered batch registered twice"
        );
        for block in &blocks.ordered_blocks {
            let store = aptos_transaction_tracing::store::TransactionTraceStore::global();
            if store.is_enabled() {
                store.record_block_stage(
                    &block.id(),
                    aptos_transaction_tracing::types::TransactionStage::BlockOrdered,
                );
            }
            // Until share queues are removed, they have already resolved these values.
            if let Some(tx) = block.pipeline_tx().lock().as_mut() {
                if let Some(sender) = tx.rand_tx.take() {
                    let _ = sender.send(block.randomness().cloned());
                }
                if let Some(sender) = tx.secret_shared_key_tx.take() {
                    let _ = sender.send(block.secret_shared_key().cloned());
                }
            }
        }
        let mut certificate = CommitCertificate::new(blocks.ordered_proof.commit_info().clone());
        if let Some(votes) = self.pending_votes.remove(&round) {
            for vote in votes.into_values() {
                let _ = certificate.add_vote(vote);
            }
        }
        if let Some(proof) = self.pending_proofs.remove(&round) {
            if proof.commit_info().id() == id {
                certificate.add_decision(proof);
            }
        }
        let execution_blocks = blocks.ordered_blocks.clone();
        let hash = if self.order_vote_enabled {
            HashValue::zero()
        } else {
            blocks.ordered_proof.ledger_info().consensus_data_hash()
        };
        self.completions.push(
            async move {
                let result = async {
                    let mut ledger_info = None;
                    for block in execution_blocks {
                        let futures = block
                            .pipeline_futs()
                            .ok_or_else(|| anyhow::anyhow!("Pipeline aborted"))?;
                        let (result, elapsed, timestamp) = futures.ledger_update_fut.await?;
                        ledger_info = Some(execution_ledger_info(
                            block.block(),
                            &result,
                            timestamp,
                            hash,
                        ));
                        block.set_compute_result(result, elapsed);
                    }
                    ledger_info.ok_or_else(|| anyhow::anyhow!("Empty ordered batch").into())
                }
                .await;
                Completion::Executed(id, result)
            }
            .boxed(),
        );
        if !self.observer_enabled {
            self.completions.push(
                async move { Completion::Voted(id, last_futures.commit_vote_fut.await) }.boxed(),
            );
        }
        self.entries.insert(id, Entry {
            blocks: blocks.ordered_blocks,
            certificate,
            local_vote: None,
            broadcast: None,
        });
        self.prefix.push_back(id);
    }

    fn cache_vote(&mut self, vote: CommitVote) -> bool {
        let round = vote.commit_info().round();
        if round <= self.highest_committed_round {
            return true;
        }
        if round
            >= self
                .highest_committed_round
                .saturating_add(self.max_pending_rounds)
        {
            return false;
        }
        self.pending_votes
            .entry(round)
            .or_default()
            .insert(vote.author(), vote);
        true
    }

    fn cache_proof(&mut self, proof: LedgerInfoWithSignatures) -> bool {
        let round = proof.commit_info().round();
        if round <= self.highest_committed_round {
            return false;
        }
        self.pending_proofs.insert(round, proof);
        while self.pending_proofs.len() > 100 {
            self.pending_proofs.pop_first();
        }
        true
    }

    fn process_message(&mut self, request: IncomingCommitRequest) -> Option<HashValue> {
        let IncomingCommitRequest {
            req,
            protocol,
            response_sender,
        } = request;
        let (id, accepted) = match req {
            CommitMessage::Vote(vote) => {
                let id = vote.commit_info().id();
                let accepted = if let Some(entry) = self.entries.get_mut(&id) {
                    entry.certificate.add_vote(vote)
                } else {
                    self.cache_vote(vote)
                };
                (id, accepted)
            },
            CommitMessage::Decision(proof) => {
                let id = proof.ledger_info().commit_info().id();
                let accepted = if let Some(entry) = self.entries.get_mut(&id) {
                    entry.certificate.add_decision(proof.into_inner());
                    true
                } else {
                    self.cache_proof(proof.into_inner())
                };
                (id, accepted)
            },
            CommitMessage::Ack(_) | CommitMessage::Nack => {
                error!("Unexpected commit acknowledgement");
                return None;
            },
        };
        reply(protocol, response_sender, accepted);
        accepted.then_some(id)
    }

    async fn try_commit(&mut self, id: HashValue) {
        let Some(proof) = self
            .entries
            .get_mut(&id)
            .and_then(|entry| entry.certificate.certificate(&self.epoch_state.verifier))
        else {
            return;
        };
        let mut blocks = vec![];
        while let Some(head) = self.prefix.pop_front() {
            let entry = self.entries.remove(&head).expect("Prefix entry must exist");
            blocks.extend(entry.blocks);
            if head == id {
                break;
            }
        }
        assert_eq!(blocks.last().map(|block| block.id()), Some(id));
        observe_block(
            blocks.last().expect("Nonempty prefix").timestamp_usecs(),
            BlockStage::COMMIT_CERTIFIED,
        );
        if let Some(publisher) = &self.publisher {
            publisher.publish_message(ConsensusObserverMessage::new_commit_decision_message(
                proof.clone(),
            ));
        }
        for block in &blocks {
            let store = aptos_transaction_tracing::store::TransactionTraceStore::global();
            if store.is_enabled() {
                store.record_block_stage(
                    &block.id(),
                    aptos_transaction_tracing::types::TransactionStage::Certified,
                );
            }
            self.pending_commit_blocks
                .insert(block.round(), block.clone());
            if let Some(tx) = block.pipeline_tx().lock().as_mut() {
                if let Some(sender) = tx.commit_proof_tx.take() {
                    let _ = sender.send(proof.clone());
                }
            }
        }
        let round = proof.commit_info().round();
        let ends_epoch = proof.ledger_info().ends_epoch();
        let network = self.network.clone();
        // Capture futures before pruning/reset can detach them from the block.
        let commit_futures: Vec<_> = blocks
            .iter()
            .map(|block| {
                block
                    .pipeline_futs()
                    .expect("Committing block has futures")
                    .commit_ledger_fut
            })
            .collect();
        self.commits.push(
            async move {
                let result = async {
                    for future in commit_futures {
                        future.await?;
                    }
                    if ends_epoch {
                        network
                            .send_epoch_change(EpochChangeProof::new(vec![proof], false))
                            .await;
                    }
                    // Retain the entire prefix until persistence has completed.
                    drop(blocks);
                    Ok(())
                }
                .await;
                (round, result)
            }
            .boxed(),
        );
        self.previous_commit_time = Instant::now();
        if ends_epoch {
            self.drain().await;
        }
    }

    async fn drain(&mut self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        // Proof-delivered blocks must finish before any suffix is cancelled.
        for block in self.pending_commit_blocks.values() {
            block.wait_for_commit_ledger().await;
        }
        // Storage futures are not abortable and can still await their parents.
        // Cancel the entire suffix (including queued blocks) before waiting:
        // entries are unordered, so waiting on a child here can prevent us from
        // ever reaching the parent whose cancellation would unblock it.
        let mut aborted = vec![];
        for entry in self.entries.values() {
            for block in &entry.blocks {
                if let Some(futures) = block.abort_pipeline() {
                    aborted.push(futures);
                }
            }
        }
        while let Ok(Some(blocks)) = self.block_rx.try_next() {
            for block in blocks.ordered_blocks {
                if let Some(futures) = block.abort_pipeline() {
                    aborted.push(futures);
                }
            }
        }
        for futures in aborted {
            futures.wait_until_finishes().await;
        }
        // Cancel only observation of execution/signing; these never own storage work.
        self.completions = FuturesUnordered::new();
        // Complete pending commit listeners, including epoch-change publication.
        while let Some((round, result)) = self.commits.next().await {
            self.record_commit(round, result);
        }
        self.entries.clear();
        self.prefix.clear();
        self.pending_commit_blocks.clear();
        self.previous_commit_time = Instant::now();
    }

    fn record_commit(&mut self, round: Round, result: TaskResult<()>) {
        match result {
            Ok(()) => {
                self.highest_committed_round = self.highest_committed_round.max(round);
                self.pending_votes
                    .retain(|r, _| *r > self.highest_committed_round);
                self.pending_proofs
                    .retain(|r, _| *r > self.highest_committed_round);
                self.pending_commit_blocks
                    .retain(|r, _| *r > self.highest_committed_round);
            },
            Err(error) => error!("Commit pipeline failed at round {}: {}", round, error),
        }
    }

    fn rebroadcast(&mut self) {
        if self.previous_commit_time.elapsed() < Duration::from_millis(BROADCAST_INTERVAL_MS) {
            return;
        }
        let retry: Vec<_> = self
            .prefix
            .iter()
            .filter_map(|id| {
                let entry = self.entries.get(id)?;
                let vote = entry.local_vote.as_ref()?;
                if entry.broadcast.as_ref().is_none_or(|(started, _)| {
                    started.elapsed() >= Duration::from_millis(REBROADCAST_INTERVAL_MS)
                }) {
                    Some((*id, vote.clone()))
                } else {
                    None
                }
            })
            .collect();
        for (id, vote) in retry {
            let broadcast = self.broadcast(vote);
            if let Some(entry) = self.entries.get_mut(&id) {
                entry.broadcast = broadcast;
            }
        }
    }

    fn update_metrics(&self) {
        let ordered = self
            .entries
            .values()
            .filter(|entry| !entry.certificate.has_execution_result())
            .count();
        let signed = self
            .entries
            .values()
            .filter(|entry| entry.local_vote.is_some())
            .count();
        let executed = self
            .entries
            .values()
            .filter(|entry| entry.certificate.has_execution_result() && entry.local_vote.is_none())
            .count();
        for (phase, count) in [
            ("ordered", ordered),
            ("executed", executed),
            ("signed", signed),
            ("aggregated", 0),
        ] {
            counters::NUM_BLOCKS_IN_PIPELINE
                .with_label_values(&[phase])
                .set(count as i64);
        }
    }

    pub async fn start(mut self) {
        let (verified_tx, mut verified_rx) = unbounded();
        let mut commit_rx = self.commit_rx.take().expect("Commit receiver exists");
        let epoch = self.epoch_state.clone();
        let executor = self.bounded_executor.clone();
        let generation = self.generation.clone();
        let verification = tokio::spawn(async move {
            while let Some((author, request)) = commit_rx.next().await {
                let tx = verified_tx.clone();
                let epoch = epoch.clone();
                let current = generation.load(Ordering::SeqCst);
                executor
                    .spawn_blocking(move || match request.req.verify(author, &epoch.verifier) {
                        Ok(()) => {
                            let _ = tx.unbounded_send((current, request));
                        },
                        Err(error) => warn!("Invalid commit message: {}", error),
                    })
                    .await;
            }
        });
        let mut interval = tokio::time::interval(Duration::from_millis(BROADCAST_INTERVAL_MS));
        loop {
            tokio::select! {
                Some(blocks) = self.block_rx.next() => self.register(blocks),
                Some(request) = self.reset_rx.next() => {
                    let stop = matches!(request.signal, ResetSignal::Stop);
                    self.drain().await;
                    if let ResetSignal::TargetRound(round) = request.signal {
                        self.highest_committed_round = self.highest_committed_round.max(round);
                        self.pending_votes.retain(|r, _| *r > self.highest_committed_round);
                        self.pending_proofs.retain(|r, _| *r > self.highest_committed_round);
                    }
                    let _ = request.tx.send(ResetAck::default());
                    if stop { break; }
                },
                Some((generation, request)) = verified_rx.next() => {
                    if generation == self.generation.load(Ordering::SeqCst) {
                        if let Some(id) = self.process_message(request) { self.try_commit(id).await; }
                    }
                },
                Some(completion) = self.completions.next(), if !self.completions.is_empty() => match completion {
                    Completion::Executed(id, Ok(info)) => {
                        if let Some(entry) = self.entries.get_mut(&id) { entry.certificate.set_execution_result(info); }
                        self.try_commit(id).await;
                    },
                    Completion::Voted(id, Ok(vote)) => {
                        if self.entries.contains_key(&id) {
                            let broadcast = self.broadcast(vote.clone());
                            let entry = self.entries.get_mut(&id).expect("Entry exists");
                            entry.local_vote = Some(vote);
                            entry.broadcast = broadcast;
                        }
                    },
                    Completion::Executed(id, Err(error)) | Completion::Voted(id, Err(error)) => {
                        if !error.is_cancellation() { error!("Pipeline block {} failed: {}", id, error); }
                    },
                },
                Some((round, result)) = self.commits.next(), if !self.commits.is_empty() => self.record_commit(round, result),
                _ = interval.tick() => { self.update_metrics(); self.rebroadcast(); },
            }
        }
        verification.abort();
        let _ = verification.await;
    }
}

fn reply(
    protocol: ProtocolId,
    sender: oneshot::Sender<Result<Bytes, aptos_network::protocols::rpc::error::RpcError>>,
    accepted: bool,
) {
    let response = ConsensusMsg::CommitMessage(Box::new(
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

#[cfg(test)]
#[path = "tests/commit_pipeline_reset_tests.rs"]
mod reset_tests;
