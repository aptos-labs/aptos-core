// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    framework::{
        crypto,
        crypto::{dummy_signature, SignatureVerifier, Signer},
        module_network::ModuleId,
        network::{MessageCertifier, MessageVerifier, NetworkSender, NetworkService},
        timer::TimerService,
        ContextFor, NodeId, Protocol,
    },
    metrics,
    metrics::Sender,
    protocol,
    raikou::{
        dissemination::{
            penalty_tracker,
            penalty_tracker::{PenaltyTracker, PenaltyTrackerReports},
            DisseminationLayer, FullBlockAvailable, Kill, Metrics, NewQCWithPayload,
            ProposalReceived,
        },
        types::*,
    },
};
use anyhow::Context;
use aptos_bitvec::BitVec;
use aptos_crypto::{bls12381::Signature, hash::CryptoHash, Genesis};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use defaultmap::DefaultBTreeMap;
use itertools::Itertools;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet, VecDeque},
    future::Future,
    sync::{atomic::AtomicBool, Arc},
    time::{Duration, SystemTime},
};
use tokio::{sync::RwLock, time::Instant};

#[derive(Clone, Serialize, Deserialize)]
#[serde(from = "BatchSerialization")]
pub struct Batch {
    data: BatchData,
    signature: Signature,

    #[serde(skip)]
    digest: BatchHash,
}

#[derive(Deserialize)]
struct BatchSerialization {
    data: BatchData,
    signature: Signature,
}

impl From<BatchSerialization> for Batch {
    fn from(serialized: BatchSerialization) -> Self {
        Self {
            digest: serialized.data.hash(),
            data: serialized.data,
            signature: serialized.signature,
        }
    }
}

#[derive(Clone, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
struct BatchSignatureData {
    digest: BatchHash,
}

#[derive(Clone, CryptoHasher, BCSCryptoHash, Serialize, Deserialize)]
struct BatchData {
    author: NodeId,
    batch_id: BatchId,
    txns: Arc<Vec<Txn>>,
}

impl Batch {
    pub fn get_info(&self) -> BatchInfo {
        BatchInfo {
            author: self.author(),
            batch_id: self.batch_id(),
            digest: self.digest.clone(),
        }
    }

    pub fn author(&self) -> NodeId {
        self.data.author
    }

    pub fn batch_id(&self) -> BatchId {
        self.data.batch_id
    }

    pub fn txns(&self) -> &[Txn] {
        &self.data.txns
    }

    pub fn verify(&self, verifier: &SignatureVerifier) -> anyhow::Result<()> {
        let sig_data = &BatchSignatureData {
            digest: self.digest.clone(),
        };
        verifier.verify(self.author(), sig_data, &self.signature)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub enum Message {
    Batch(Batch),
    PoAVote(BatchId, BatchHash, Signature),
    AvailabilityCert(PoA),
    Fetch(Vec<BatchHash>),
    FetchResp(Vec<Batch>),
    PenaltyTrackerReport(Round, PenaltyTrackerReports),
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Batch(batch) => write!(f, "Batch({})", batch.batch_id()),
            Message::PoAVote(batch_id, _, _) => write!(f, "BatchStored({})", batch_id),
            Message::AvailabilityCert(poa) => write!(f, "AvailabilityCert({})", poa.info.batch_id),
            Message::Fetch(digests) => write!(f, "Fetch({} batches)", digests.len()),
            Message::FetchResp(batches) => write!(f, "FetchResp({} batches)", batches.len()),
            Message::PenaltyTrackerReport(round, reports) => {
                write!(f, "PenaltyTrackerReport({}, {:?})", round, reports)
            },
        }
    }
}

pub struct Certifier {
    signer: Signer,
}

impl Certifier {
    pub fn new(signer: Signer) -> Self {
        Self { signer }
    }
}

impl MessageCertifier for Certifier {
    type Message = Message;

    async fn certify(&self, message: &mut Self::Message) -> anyhow::Result<()> {
        match message {
            Message::Batch(_batch) => {},
            Message::PoAVote(_batch_id, batch_digest, signature) => {
                *signature = self
                    .signer
                    .sign(&PoAVoteSignatureData {
                        batch_digest: batch_digest.clone(),
                    })
                    .unwrap();
            },
            Message::AvailabilityCert(_) => {},
            Message::Fetch(_) => {},
            Message::FetchResp(_) => {},
            Message::PenaltyTrackerReport(_, _) => {},
        }

        Ok(())
    }
}

pub struct Verifier {
    sig_verifier: SignatureVerifier,
    config: Config,
    my_batches: Arc<RwLock<BTreeMap<BatchId, BatchHash>>>,
}

impl Verifier {
    pub async fn new<TI>(diss_layer: &NativeDisseminationLayer<TI>) -> Self {
        let inner = diss_layer.inner.lock().await;
        Self {
            sig_verifier: inner.sig_verifier.clone(),
            config: diss_layer.config.clone(),
            my_batches: inner.my_batches.clone(),
        }
    }
}

impl MessageVerifier for Verifier {
    type Message = Message;

    async fn verify(&self, sender: NodeId, message: &Self::Message) -> anyhow::Result<()> {
        match message {
            Message::Batch(batch) => {
                if batch.author() != sender {
                    return Err(anyhow::anyhow!("Batch author does not match the sender."));
                }
                batch.verify(&self.sig_verifier).context("Invalid batch")
            },

            Message::PoAVote(batch_id, batch_digest, signature) => {
                let Some(digest) = self.my_batches.read().await.get(batch_id).cloned() else {
                    return Err(anyhow::anyhow!(
                        "PoAVote for an unknown batch id {}",
                        batch_id
                    ));
                };

                if digest != *batch_digest {
                    return Err(anyhow::anyhow!("Invalid batch digest in PoAVote"));
                }

                let sig_data = PoAVoteSignatureData {
                    batch_digest: digest.clone(),
                };

                self.sig_verifier
                    .verify(sender, &sig_data, signature)
                    .context("Invalid signature in PoAVote")
            },

            Message::AvailabilityCert(poa) => poa
                .verify(&self.sig_verifier, self.config.poa_quorum)
                .context("Invalid PoA"),

            Message::Fetch(_) => Ok(()),

            Message::FetchResp(batches) => {
                for batch in batches.iter() {
                    batch.verify(&self.sig_verifier)?;
                }

                Ok(())
            },

            Message::PenaltyTrackerReport(_round, _reports) => Ok(()),
        }
    }
}

#[derive(Clone)]
pub enum TimerEvent {
    NewBatch(BatchId),
    PenaltyTrackerReport(NodeId, Round, Instant, Payload),
    Status,
}

#[derive(Clone)]
pub struct BlockSizeLimit {
    poa_size: usize,
    batch_size: usize,
    byte_limit: usize,
}

impl BlockSizeLimit {
    pub fn from_max_number_of_poas(poa_limit: usize, n_nodes: usize) -> Self {
        let dummy_batch_info = BatchInfo {
            author: n_nodes,
            batch_id: 1234,
            digest: BatchHash::random(),
        };

        let batch_size = bcs::to_bytes(&dummy_batch_info).unwrap().len();

        let dummy_poa = PoA {
            info: dummy_batch_info,
            signers: BitVec::from(vec![true; n_nodes]),
            multi_signature: dummy_signature(),
        };

        let poa_size = bcs::to_bytes(&dummy_poa).unwrap().len();

        Self {
            poa_size,
            batch_size,
            byte_limit: poa_limit * poa_size,
        }
    }

    pub fn poa_limit(&self) -> usize {
        self.byte_limit / self.poa_size
    }

    pub fn batch_limit(&self, n_poas: usize) -> usize {
        (self.byte_limit - n_poas * self.poa_size) / self.batch_size
    }
}

#[derive(Clone)]
pub struct Config {
    pub module_id: ModuleId,
    pub n_nodes: usize,
    pub f: usize,
    pub poa_quorum: usize,
    pub delta: Duration,
    pub batch_interval: Duration,
    pub batch_fetch_interval: Duration,
    pub batch_fetch_multiplicity: usize,
    pub enable_optimistic_dissemination: bool,
    pub enable_penalty_tracker: bool,
    pub penalty_tracker_report_delay: Duration,
    pub status_interval: Duration,
    pub block_size_limit: BlockSizeLimit,
}

#[derive(Clone)]
pub struct NativeDisseminationLayer<TI> {
    config: Config,
    inner: Arc<tokio::sync::Mutex<NativeDisseminationLayerProtocol<TI>>>,
}

impl<TI> NativeDisseminationLayer<TI>
where
    TI: Iterator<Item = Vec<Txn>> + Send + Sync,
{
    pub fn new(
        node_id: NodeId,
        mut config: Config,
        txns_iter: TI,
        consensus_module_id: ModuleId,
        detailed_logging: bool,
        metrics: Metrics,
        signer: Signer,
        verifier: SignatureVerifier,
        execute_tx: tokio::sync::mpsc::Sender<Batch>,
    ) -> Self {
        if !config.enable_optimistic_dissemination && !config.enable_penalty_tracker {
            aptos_logger::warn!(
                "Disabling the penalty tracker because optimistic dissemination is disabled."
            );
            config.enable_penalty_tracker = false;
        }

        Self {
            config: config.clone(),
            inner: Arc::new(tokio::sync::Mutex::new(
                NativeDisseminationLayerProtocol::new(
                    node_id,
                    config,
                    txns_iter,
                    consensus_module_id,
                    detailed_logging,
                    metrics,
                    signer,
                    verifier,
                    execute_tx,
                ),
            )),
        }
    }

    pub fn protocol(
        &self,
    ) -> Arc<tokio::sync::Mutex<impl Protocol<Message = Message, TimerEvent = TimerEvent>>> {
        self.inner.clone()
    }
}

impl<TI> DisseminationLayer for NativeDisseminationLayer<TI>
where
    TI: Iterator<Item = Vec<Txn>> + Send + Sync + 'static,
{
    fn module_id(&self) -> ModuleId {
        self.config.module_id
    }

    async fn prepare_block(
        &self,
        round: Round,
        exclude: HashSet<BatchInfo>,
        _missing_authors: Option<BitVec>,
    ) -> Payload {
        let mut inner = self.inner.lock().await;

        let mut poas: Vec<PoA> = inner
            .uncommitted_poas
            .iter()
            .filter(|&(_batch_digest, poa)| !exclude.contains(poa.info()))
            .map(|(_batch_digest, poa)| poa.clone())
            .collect();

        let limit = inner.config.block_size_limit.poa_limit();
        if poas.len() > limit {
            aptos_logger::warn!(
                "Block size limit reached: {} PoAs, {} allowed",
                poas.len(),
                limit
            );
            poas.truncate(limit);
        }

        let batches = if inner.config.enable_optimistic_dissemination && poas.len() < limit {
            let mut batches: Vec<BatchInfo> = inner
                .uncommitted_uncertified_batches
                .iter()
                .map(|batch_hash| inner.batches[batch_hash].get_info())
                .filter(|batch_info| !exclude.contains(batch_info))
                .collect();

            let limit = inner.config.block_size_limit.batch_limit(poas.len());
            if batches.len() > limit {
                aptos_logger::warn!(
                    "Block size limit reached: {} batches, {} allowed",
                    batches.len(),
                    limit
                );
                batches.truncate(limit);
            }

            // If the penalty tracker is disabled, this will sort the batches
            // by the order they were received.
            inner.penalty_tracker.prepare_new_block(round, batches)
        } else {
            Default::default()
        };

        Payload::new(round, inner.node_id, poas, batches)
    }

    async fn available_prefix(&self, payload: &Payload, _cached_value: usize) -> (Prefix, BitVec) {
        let inner = self.inner.lock().await;

        let mut missing_authors = BitVec::with_num_bits(inner.config.n_nodes as u16);
        let mut prefix = N_SUB_BLOCKS;

        for (i, sub_block) in payload.sub_blocks().enumerate() {
            for batch_info in sub_block {
                if !inner.batches.contains_key(&batch_info.digest) {
                    missing_authors.set(batch_info.author as u16);
                    if prefix == N_SUB_BLOCKS {
                        prefix = i;
                    }
                }
            }
        }

        if prefix == N_SUB_BLOCKS {
            assert!(missing_authors.all_zeros());
        }

        (prefix, missing_authors)
    }

    async fn notify_commit(&self, payloads: Vec<Payload>, _block_timestamp: u64) {
        let mut inner = self.inner.lock().await;
        let now = Instant::now();

        for payload in &payloads {
            for batch in payload.all() {
                if inner.committed_batches.contains(&batch.digest) {
                    // NB: This may happen because de-duplication is best-effort:
                    // e.g., if the block for the parent QC is not available, we will
                    // go ahead with an incomplete `exclude` set.
                    aptos_logger::warn!(
                        "Duplicate commit for batch {} (digest: {:#x})",
                        batch.batch_id,
                        batch.digest,
                    );
                    continue;
                }

                inner.committed_batches.insert(batch.digest.clone());
                inner.uncommitted_poas.remove(&batch.digest);
                inner.uncommitted_uncertified_batches.remove(&batch.digest);

                inner.batch_commit_time.insert(batch.digest.clone(), now);

                if batch.author == inner.node_id {
                    let commit_time = inner.to_deltas(now - inner.batch_send_time[&batch.digest]);
                    inner.metrics.batch_commit_time.push((now, commit_time));
                }
            }
        }

        // Metrics:
        // Only track queueing time and penalties for the committed batches.
        // At the moment, they are only tracked for optimistically committed batches.
        for payload in &payloads {
            for batch in payload.sub_blocks().flatten() {
                if payload.author() == inner.node_id {
                    let block_prepare_time =
                        inner.penalty_tracker.block_prepare_time(payload.round());
                    let batch_receive_time = inner
                        .penalty_tracker
                        .batch_receive_time(batch.digest.clone());
                    let penalty = inner
                        .penalty_tracker
                        .block_prepare_penalty(payload.round(), batch.author);
                    let batch_propose_delay = block_prepare_time - batch_receive_time;

                    assert!(batch_propose_delay >= penalty);
                    let queueing_time_in_deltas = inner.to_deltas(batch_propose_delay);
                    inner
                        .metrics
                        .queueing_time
                        .push((now, queueing_time_in_deltas));

                    let penalty_in_deltas = inner.to_deltas(penalty);
                    inner
                        .metrics
                        .penalty_wait_time
                        .push((now, penalty_in_deltas));
                }
            }
        }

        inner.execution_queue.extend(
            payloads
                .iter()
                .flat_map(|payload| payload.all())
                .map(|batch_info| batch_info.digest.clone()),
        );
        inner.execute_prefix().await;
    }

    async fn set_first_committed_block_timestamp(&self, timestamp: SystemTime) {
        self.inner.lock().await.first_committed_block_timestamp = Some(timestamp);
    }
}

#[derive(Clone)]
struct FetchTaskHandle {
    kill: Arc<AtomicBool>,
}

impl FetchTaskHandle {
    fn new() -> Self {
        Self {
            kill: Arc::new(AtomicBool::new(false)),
        }
    }

    fn kill(&self) {
        self.kill.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    fn is_killed(&self) -> bool {
        self.kill.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[derive(Default)]
struct CurrentProposalStatus {
    round: Round,
    missing_batches: HashSet<BatchHash>,
}

pub struct NativeDisseminationLayerProtocol<TI> {
    txns_iter: TI,
    config: Config,
    node_id: NodeId,

    penalty_tracker: PenaltyTracker,

    // Storage for all received batches.
    batches: BTreeMap<BatchHash, Batch>,
    // Batches currently being fetched and the flags to notify them to stop.
    fetch_tasks: BTreeMap<BatchHash, FetchTaskHandle>,
    // List of batches created by this node.
    my_batches: Arc<RwLock<BTreeMap<BatchId, BatchHash>>>,
    // Set of committed batches.
    committed_batches: BTreeSet<BatchHash>,
    // Set of known PoAs that are not yet committed.
    uncommitted_poas: BTreeMap<BatchHash, PoA>,
    // Set of known uncertified batches that are not yet committed.
    uncommitted_uncertified_batches: BTreeSet<BatchHash>,

    // The set of nodes that have stored this node's batch with the given sequence number.
    batch_stored_votes: DefaultBTreeMap<BatchId, BTreeMap<NodeId, Signature>>,

    // Tracking the missing batches in the current proposal.
    current_proposal_status: CurrentProposalStatus,
    consensus_module_id: ModuleId,

    // Crypto
    signer: Signer,
    sig_verifier: SignatureVerifier,

    // Execution
    execute_tx: tokio::sync::mpsc::Sender<Batch>,

    // Logging and metrics
    detailed_logging: bool,
    first_committed_block_timestamp: Option<SystemTime>,
    metrics: Metrics,
    batch_send_time: BTreeMap<BatchHash, Instant>,
    batch_commit_time: BTreeMap<BatchHash, Instant>,
    execution_queue: VecDeque<BatchHash>,
}

impl<TI> NativeDisseminationLayerProtocol<TI> {
    fn to_deltas(&self, duration: Duration) -> f64 {
        duration.as_secs_f64() / self.config.delta.as_secs_f64()
    }

    fn time_in_delta(&self) -> Option<f64> {
        Some(
            self.to_deltas(
                SystemTime::now()
                    .duration_since(self.first_committed_block_timestamp?)
                    .ok()?,
            ),
        )
    }

    fn log_info(&self, msg: String) {
        let time_str = self
            .time_in_delta()
            .map(|t| format!("{:.2}Δ", t))
            .unwrap_or_else(|| "???Δ".to_string());

        aptos_logger::info!(
            "Node {} at {}: Dissemination Layer: {}",
            self.node_id,
            time_str,
            msg
        );
    }

    fn log_detail(&self, msg: String) {
        if self.detailed_logging {
            self.log_info(msg);
        }
    }
}

impl<TI> NativeDisseminationLayerProtocol<TI>
where
    TI: Iterator<Item = Vec<Txn>> + Send + Sync,
{
    pub fn new(
        node_id: NodeId,
        config: Config,
        txns_iter: TI,
        consensus_module_id: ModuleId,
        detailed_logging: bool,
        metrics: Metrics,
        signer: Signer,
        sig_verifier: SignatureVerifier,
        execute_tx: tokio::sync::mpsc::Sender<Batch>,
    ) -> Self {
        let penalty_tracker_config = penalty_tracker::Config {
            n_nodes: config.n_nodes,
            f: config.f,
            enable: config.enable_penalty_tracker,
            batch_expiration_time: config.delta * 3,
        };

        Self {
            txns_iter,
            config,
            node_id,
            penalty_tracker: PenaltyTracker::new(node_id, penalty_tracker_config, detailed_logging),
            batches: BTreeMap::new(),
            fetch_tasks: Default::default(),
            my_batches: Default::default(),
            committed_batches: BTreeSet::new(),
            uncommitted_poas: BTreeMap::new(),
            uncommitted_uncertified_batches: BTreeSet::new(),
            batch_stored_votes: Default::default(),
            current_proposal_status: Default::default(),
            consensus_module_id,
            execution_queue: Default::default(),
            detailed_logging,
            first_committed_block_timestamp: None,
            metrics,
            batch_send_time: Default::default(),
            batch_commit_time: Default::default(),
            signer,
            sig_verifier,
            execute_tx,
        }
    }

    async fn on_new_batch(&mut self, batch: Batch, fetched: bool, ctx: &mut impl ContextFor<Self>) {
        let digest = batch.digest.clone();
        let batch_id = batch.batch_id();
        let author = batch.author();

        // NB: it may happen that the same batch is received multiple times.
        self.batches.insert(digest.clone(), batch);

        if !self.current_proposal_status.missing_batches.is_empty() {
            self.current_proposal_status.missing_batches.remove(&digest);
            if self.current_proposal_status.missing_batches.is_empty() {
                ctx.notify(self.consensus_module_id, FullBlockAvailable {
                    round: self.current_proposal_status.round,
                })
                .await;
            }
        }

        if let Some(handle) = self.fetch_tasks.remove(&digest) {
            handle.kill();
        }

        // NB: batches that are received ONLY through fetching will not be included in new blocks.
        if !fetched {
            self.penalty_tracker.on_new_batch(digest.clone());

            ctx.unicast(
                Message::PoAVote(
                    batch_id,
                    digest.clone(),
                    dummy_signature(), // Populated in the `sign` method.
                ),
                author,
            )
            .await;

            // Track the list of known uncommitted uncertified batches.
            if !self.uncommitted_poas.contains_key(&digest)
                && !self.committed_batches.contains(&digest)
            {
                self.uncommitted_uncertified_batches.insert(digest);
            }
        }
    }

    async fn on_new_poa(&mut self, poa: PoA, ctx: &mut impl ContextFor<Self>) {
        if !self.batches.contains_key(&poa.info.digest) {
            let signers = poa.signers.iter_ones().collect();
            // We set `override_current` to `true` because a PoA typically has more
            // signers than a QC.
            self.fetch_batch(poa.info.digest.clone(), signers, true, ctx)
                .await;
        }

        // Track the list of known uncommitted PoAs
        // and the list of known uncommitted uncertified batches.
        if !self.committed_batches.contains(&poa.info.digest) {
            self.uncommitted_uncertified_batches
                .remove(&poa.info.digest);
            self.uncommitted_poas.insert(poa.info.digest.clone(), poa);
        }
    }

    async fn fetch_batch(
        &mut self,
        digest: BatchHash,
        signers: Vec<NodeId>,
        override_current: bool,
        ctx: &mut impl ContextFor<Self>,
    ) {
        if self.batches.contains_key(&digest) {
            return;
        }

        if !override_current && self.fetch_tasks.contains_key(&digest) {
            return;
        }

        let batch_fetch_interval = self.config.batch_fetch_interval;
        let batch_fetch_multiplicity = self.config.batch_fetch_multiplicity;

        let handle = FetchTaskHandle::new();

        if let Some(old_handle) = self.fetch_tasks.insert(digest.clone(), handle.clone()) {
            old_handle.kill();
        }

        let network_sender = ctx.new_network_sender();
        tokio::spawn(async move {
            while !handle.is_killed() {
                let sample = signers
                    .choose_multiple(&mut rand::thread_rng(), batch_fetch_multiplicity)
                    .copied()
                    .collect();

                network_sender
                    .send(Message::Fetch(vec![digest.clone()]), sample)
                    .await;

                tokio::time::sleep(batch_fetch_interval).await;
            }
        });
    }

    async fn execute_prefix(&mut self) {
        let now = Instant::now();

        while let Some(batch_digest) = self.execution_queue.front() {
            let Some(batch) = self.batches.get(batch_digest) else {
                break;
            };

            let batch_digest = self.execution_queue.pop_front().unwrap();
            self.execute_tx.send(batch.clone()).await.unwrap();

            if let Some(&send_time) = self.batch_send_time.get(&batch_digest) {
                self.metrics
                    .batch_execute_time
                    .push((now, self.to_deltas(now - send_time)));
            }

            self.metrics.fetch_wait_time_after_commit.push((
                now,
                self.to_deltas(now - self.batch_commit_time[&batch_digest]),
            ));
        }
    }
}

impl<TI> Protocol for NativeDisseminationLayerProtocol<TI>
where
    TI: Iterator<Item = Vec<Txn>> + Send + Sync,
{
    type Message = Message;
    type TimerEvent = TimerEvent;

    protocol! {
        self: self;
        ctx: ctx;

        // Dissemination layer
        // In this implementation, batches are simply sent periodically, by a timer.

        upon start {
            // The first batch is sent immediately.
            ctx.set_timer(Duration::ZERO, TimerEvent::NewBatch(1));
        };

        // Creating and certifying batches

        upon timer [TimerEvent::NewBatch(batch_id)] {
            // Multicast a new batch
            let batch_data = BatchData {
                author: self.node_id,
                batch_id,
                txns: Arc::new(self.txns_iter.next().unwrap()),
            };
            let digest = batch_data.hash();
            let signature = self.signer.sign(&BatchSignatureData { digest: digest.clone() }).unwrap();

            let batch = Batch {
                data: batch_data,
                digest: digest.clone(),
                signature,
            };

            self.log_detail(format!(
                "Creating batch #{} with digest {:#x}",
                batch_id,
                digest,
            ));
            ctx.multicast(Message::Batch(batch.clone())).await;

            self.my_batches.write().await.insert(batch_id, digest.clone());
            self.on_new_batch(batch, false, ctx).await;

            // Reset the timer.
            ctx.set_timer(self.config.batch_interval, TimerEvent::NewBatch(batch_id + 1));

            self.batch_send_time.insert(digest, Instant::now());
        };

        // Upon receiving a batch, store it, reply with a BatchStored message,
        // and execute try_vote.
        upon receive [Message::Batch(batch)] from [_any_node] {
            // self.log_detail(format!(
            //     "Received batch #{} from node {} with digest {:#x}",
            //     batch.batch_id(),
            //     batch.author(),
            //     batch.digest,
            // ));

            // We call `on_new_batch` on our own batches right after we create them.
            if batch.author() != self.node_id {
                self.on_new_batch(batch, false, ctx).await;
            }
        };

        // Upon receiving a quorum of BatchStored messages for a batch,
        // form a PoA and broadcast it.
        upon receive [Message::PoAVote(batch_id, batch_digest, signature)] from node [p] {
            self.batch_stored_votes[batch_id].insert(p, signature);

            if self.batch_stored_votes[batch_id].len() == self.config.poa_quorum {
                self.log_detail(format!(
                    "Forming the PoA for batch #{} with digest {:#x}",
                    batch_id,
                    batch_digest,
                ));

                let mut signers = BitVec::with_num_bits(self.config.n_nodes as u16);
                for (node, _) in self.batch_stored_votes[batch_id].iter() {
                    signers.set(*node as u16);
                }

                let multi_signature = self.sig_verifier.aggregate_signatures(
                    self.batch_stored_votes[batch_id].values().cloned()
                ).unwrap();

                let poa = PoA {
                    info: self.batches[&batch_digest].get_info(),
                    signers,
                    multi_signature,
                };

                ctx.multicast(Message::AvailabilityCert(poa)).await;
            }
        };


        upon receive [Message::AvailabilityCert(poa)] from [_any_node] {
            self.on_new_poa(poa, ctx).await;
        };

        upon event of type [ProposalReceived] from [_any_module] {
            upon [ProposalReceived { leader, round, payload, .. }] {
                for poa in payload.poas() {
                    if !self.uncommitted_poas.contains_key(&poa.info.digest)
                        && !self.committed_batches.contains(&poa.info.digest)
                    {
                        self.on_new_poa(poa.clone(), ctx).await;
                    }
                }

                if self.config.enable_penalty_tracker {
                    ctx.set_timer(
                        self.config.penalty_tracker_report_delay,
                        TimerEvent::PenaltyTrackerReport(
                            leader,
                            round,
                            Instant::now(),
                            payload.clone(),
                        )
                    );
                }

                let missing_batches: HashSet<_> = payload
                    .sub_blocks()
                    .flatten()
                    .filter(|batch_info| !self.batches.contains_key(&batch_info.digest))
                    .map(|batch_info| batch_info.digest.clone())
                    .collect();

                if missing_batches.is_empty() {
                    ctx.notify(
                        self.consensus_module_id,
                        FullBlockAvailable { round },
                    ).await;
                }

                self.current_proposal_status = CurrentProposalStatus {
                    round,
                    missing_batches,
                }
            };
        };

        // Penalty tracker

        upon timer event [TimerEvent::PenaltyTrackerReport(leader, round, block_receive_time, payload)] {
            let reports = self.penalty_tracker.prepare_reports(payload, block_receive_time);
            ctx.unicast(Message::PenaltyTrackerReport(round, reports), leader).await;
        };

        upon receive [Message::PenaltyTrackerReport(round, reports)] from node [p] {
            if self.config.enable_penalty_tracker {
                self.penalty_tracker.register_reports(round, p, reports);
            }
        };

        // Fetching

        upon event of type [NewQCWithPayload] from [_any_module] {
            upon [NewQCWithPayload { payload, qc }] {
                for (idx, sub_block) in payload.sub_blocks().enumerate() {
                    let signers: Vec<_> = qc.vote_prefixes().sub_block_signers(idx).collect();

                    for batch in sub_block {
                        if !self.batches.contains_key(&batch.digest) {
                            self.fetch_batch(batch.digest.clone(), signers.clone(), false, ctx).await;
                        }
                    }
                }
            };
        };

        upon receive [Message::Fetch(digests)] from node [p] {
            // If receive a Fetch message, reply with the batch if it is known.
            let resp = digests.iter().filter_map(|digest| {
                self.batches.get(digest).cloned()
            }).collect();

            ctx.unicast(Message::FetchResp(resp), p).await;
        };

        upon receive [Message::FetchResp(batches)] from [_any_node] {
            // If receive a FetchResp message, store the batches.
            for batch in batches {
                if !self.fetch_tasks.contains_key(&batch.digest) {
                    // Either we already received the batch or we never requested it.
                    continue;
                }

                self.on_new_batch(batch, true, ctx).await;
            }
            self.execute_prefix().await;
        };

        // Logging and halting

        upon start {
            self.log_detail("Started".to_string());
            ctx.set_timer(self.config.status_interval, TimerEvent::Status);
        };

        upon event of type [Kill] from [_any_module] {
            upon [Kill()] {
                self.log_detail("Halting by Kill event".to_string());
                ctx.halt();

                for handle in self.fetch_tasks.values() {
                    handle.kill();
                }
            };
        };

        upon timer [TimerEvent::Status] {
            self.log_detail(format!(
                "STATUS:\n\
                \tbatches produced: {}\n\
                \tbatches stored: {}\n\
                \tbatches committed: {}\n\
                \tuncommitted_poas.len(): {}\n\
                \tuncommitted_uncertified_batches.len(): {}\n\
                \texecution_queue.len(): {}\n\
                \tactive fetch tasks: {}\n",
                self.my_batches.read().await.len(),
                self.batches.len(),
                self.committed_batches.len(),
                self.uncommitted_poas.len(),
                self.uncommitted_uncertified_batches.len(),
                self.execution_queue.len(),
                self.fetch_tasks.len(),
            ));
            ctx.set_timer(self.config.status_interval, TimerEvent::Status);
        };
    }
}

impl<TI> Drop for NativeDisseminationLayerProtocol<TI> {
    fn drop(&mut self) {
        self.log_detail("Halting by Drop".to_string());

        for handle in self.fetch_tasks.values() {
            handle.kill();
        }
    }
}
