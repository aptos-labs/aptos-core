// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    framework::{
        self,
        crypto::{SignatureVerifier, Signer},
        network::MessageVerifier,
        ContextFor, NodeId, Protocol,
    },
    metrics::{self, Sender},
    monitor, protocol,
    raptr::{
        counters::{
            BLOCK_TRACING, PREFIX_VOTED_PREVIOUSLY_COUNTER, QC_PREFIX_HISTOGRAM,
            QC_TIMER_VOTE_FULLBLOCK_COUNTER, QC_VOTING_PREFIX_HISTOGRAM,
            RAIKOU_BATCH_CONSENSUS_LATENCY, RAIKOU_BLOCK_COMMIT_RATE,
            RAIKOU_BLOCK_CONSENSUS_LATENCY, ROUND_ENTER_REASON,
        },
        dissemination::{
            self, DisseminationLayer, FullBlockAvailable, Kill, NewQCWithPayload, ProposalReceived,
        },
        types::*,
    },
};
use anyhow::{ensure, Context};
use aptos_bitvec::BitVec;
use aptos_consensus_types::{
    common::Author, payload::BatchPointer, proof_of_store::ProofCache,
    round_timeout::RoundTimeoutReason,
};
use aptos_crypto::{bls12381::Signature, hash::CryptoHash};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use aptos_types::{
    aggregate_signature::{AggregateSignature, PartialSignatures},
    validator_verifier::ValidatorVerifier,
};
use defaultmap::DefaultBTreeMap;
use futures_channel::mpsc::UnboundedSender;
use itertools::Itertools;
use mini_moka::sync::Cache;
use nanovec::NanoArrayBit;
use rand::prelude::SliceRandom;
use serde::{ser::SerializeTuple, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    cmp::{max, max_by, max_by_key, min, Ordering},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::{Debug, Display, Formatter},
    num::NonZeroU8,
    ops::{BitOr, Deref},
    sync::{Arc, Mutex},
    task::ready,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::time::Instant;

#[derive(Clone, Serialize, Deserialize)]
pub enum Message {
    // Consensus
    Propose(Block),
    QcVote(Round, Prefix, BlockHash, Signature, BitVec),
    CcVote(QC, Signature),
    TcVote(Round, QC, Signature, RoundTimeoutReason),
    AdvanceRound(Round, RoundEntryReason),
    FetchReq(BlockHash),
    FetchResp(Block),
}

impl Debug for Message {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Message::Propose(block) => write!(f, "Propose({})", block.round()),
            Message::QcVote(round, _, _, _, _) => write!(f, "QcVote({})", round),
            Message::CcVote(qc, _) => {
                write!(f, "CcVote({})", qc.round())
            },
            Message::TcVote(round, _, _, _) => {
                write!(f, "TcVote(round {})", round)
            },
            Message::AdvanceRound(round, reason) => {
                write!(f, "AdvanceRound({:?}, {:?})", round, reason)
            },
            Message::FetchReq(block_hash) => write!(f, "FetchReq({})", block_hash),
            Message::FetchResp(block) => write!(f, "FetchResp({})", block.round()),
        }
    }
}

// All signatures are done in-line in the protocol.
pub type Certifier = framework::network::NoopCertifier<Message>;

pub struct Verifier {
    pub config: Config,
    pub sig_verifier: SignatureVerifier,
    pub proof_cache: ProofCache,
}

impl Verifier {
    pub fn new<DL>(protocol: &RaptrNode<DL>, proof_cache: ProofCache) -> Self {
        Verifier {
            config: protocol.config.clone(),
            sig_verifier: protocol.sig_verifier.clone(),
            proof_cache,
        }
    }
}

impl MessageVerifier for Verifier {
    type Message = Message;

    async fn verify(&self, sender: NodeId, message: &Self::Message) -> anyhow::Result<()> {
        match message {
            Message::Propose(block) => monitor!("verify_propose", {
                ensure!(block.author() == sender, "Propose message from non-author");

                block
                    .verify(self)
                    .context("Error verifying the block in Propose message")
            }),
            Message::QcVote(round, prefix, block_digest, signature, _) => {
                monitor!("verify_qcvote", {
                    ensure!(*prefix <= N_SUB_BLOCKS, "Invalid prefix in QcVote message");

                    self.sig_verifier.verify_tagged(
                        sender,
                        &QcVoteSignatureCommonData {
                            round: *round,
                            block_digest: block_digest.clone(),
                        },
                        *prefix,
                        signature,
                    )
                })
            },
            Message::CcVote(qc, signature) => monitor!("verify_ccvote", {
                self.sig_verifier
                    .verify_tagged(
                        sender,
                        &CcVoteSignatureCommonData {
                            round: qc.round(),
                            block_digest: *qc.block_digest(),
                        },
                        qc.prefix(),
                        signature,
                    )
                    .context("Error verifying the CC vote signature")?;

                qc.verify(self)
                    .context("Error verifying the QC in CcVote message")
            }),
            Message::AdvanceRound(round, reason) => monitor!("verify_advance", {
                reason
                    .verify(*round, self)
                    .context("Error verifying the round enter reason in AdvanceRound message")
            }),
            Message::TcVote(round, qc, signature, _) => monitor!("verify_tcvote", {
                ensure!(qc.round() <= *round, "QC in TcVote message too high");

                let sig_data = &TcVoteSignatureData {
                    timeout_round: *round,
                    qc_high_id: qc.id(),
                };

                self.sig_verifier
                    .verify(sender, sig_data, signature)
                    .context("Error verifying the TC vote signature")?;

                qc.verify(self)
                    .context("Error verifying the QC in TcVote message")
            }),
            Message::FetchReq(_block_digest) => monitor!("verify_fetchreq", Ok(())),

            Message::FetchResp(block) => monitor!(
                "verify_fetchresp",
                block
                    .verify(self)
                    .context("Error verifying the block in FetchResp message")
            ),
        }
    }
}

#[derive(Clone)]
pub enum TimerEvent {
    // Consensus
    QcVote(Round),
    Timeout(Round),
    FetchBlock(Round, BlockHash, Vec<NodeId>),

    // Other
    EndOfRun,
    Status,
    RoundSync,
}

#[derive(Clone, Debug)]
pub enum CommitReason {
    CC,
    TwoChainRule,
    Indirect,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum QcVoteReason {
    FullBlock,
    Timer,
}

#[derive(Clone, Copy)]
pub struct Config {
    pub n_nodes: usize,
    pub f: usize,
    pub storage_requirement: usize,
    pub leader_timeout: Duration,
    pub delta: Duration,
    pub enable_commit_votes: bool,

    /// The time  waits after receiving a block before voting for a QC for it
    /// if it doesn't have all the batches yet.
    pub extra_wait_before_qc_vote: Duration,
    pub enable_partial_qc_votes: bool,

    pub block_fetch_multiplicity: usize,
    pub block_fetch_interval: Duration,

    pub round_sync_interval: Duration,

    pub status_interval: Duration,
    pub end_of_run: Instant,

    /// Used for AC verification. Must be the same as in the Quorum Store module.
    pub poa_quorum: usize,
}

impl Config {
    pub fn leader(&self, round: Round) -> NodeId {
        round as usize % self.n_nodes
    }

    pub fn quorum(&self) -> usize {
        // Using more general quorum formula that works not only for n = 3f+1,
        // but for any n >= 3f+1.
        (self.n_nodes + self.f) / 2 + 1
    }
}

pub struct Metrics {
    pub block_consensus_latency: Option<metrics::UnorderedSender<(Instant, f64)>>,
    pub batch_consensus_latency: Option<metrics::UnorderedSender<(Instant, f64)>>,
}

pub trait TRaptrFailureTracker: Send + Sync {
    fn push_reason(&self, status: RoundEntryReason);
}

pub struct RaptrNode<DL> {
    node_id: NodeId,
    config: Config,
    dissemination: DL,

    // Logging and metrics
    first_committed_block_timestamp: Option<SystemTime>,
    detailed_logging: bool,
    metrics: Metrics,
    block_create_time: BTreeMap<Round, Instant>,

    // Protocol state for the pseudocode
    r_ready: Round,                 // The highest round the node is ready to enter.
    entry_reason: RoundEntryReason, // The justification for entering the round r_read.
    r_cur: Round,                   // The current round the node is in.
    r_timeout: Round,               // The highest round the node has voted to time out.
    last_qc_vote: SubBlockId,
    cc_voted: DefaultBTreeMap<Round, bool>, // Whether the node has CC-voted in round r.
    qc_high: QC,
    committed_qc: QC,

    // Additional variables necessary for an efficient implementation

    // Set of already processed QCs.
    known_qcs: BTreeSet<SubBlockId>,

    // Map from a QC id to the list of blocks that wait for this QC to be satisfied.
    pending_blocks: DefaultBTreeMap<SubBlockId, Vec<BlockHash>>,

    // Set of blocks for which we have the full causal history available.
    satisfied_blocks: BTreeSet<BlockHash>,

    // Map from an unsatisfied block hash to the list of QCs for this block.
    pending_qcs: DefaultBTreeMap<BlockHash, Vec<QC>>,

    // Map from an unavailable block hash to the list of QCs for this block.
    qcs_without_blocks: DefaultBTreeMap<BlockHash, Vec<QC>>,

    // QCs for which we have the full causal history available.
    satisfied_qcs: BTreeSet<SubBlockId>,

    // QCs that are committed via direct commit votes, for which we do not yet have their full causal history available.
    qcs_to_commit: BTreeMap<SubBlockId, (QC, CommitReason)>,

    leader_proposal: BTreeMap<Round, BlockHash>,
    blocks: BTreeMap<BlockHash, Block>,
    available_prefix_cache: SubBlockId,
    qc_votes: DefaultBTreeMap<
        Round,
        DefaultBTreeMap<BlockHash, BTreeMap<NodeId, (Prefix, Signature, BitVec)>>,
    >,
    full_prefix_votes: DefaultBTreeMap<Round, DefaultBTreeMap<BlockHash, BTreeSet<NodeId>>>,
    received_cc_vote: DefaultBTreeMap<Round, BTreeSet<NodeId>>,
    cc_votes: DefaultBTreeMap<Round, BTreeMap<(Prefix, NodeId), (QC, Signature)>>,
    tc_votes: DefaultBTreeMap<Round, BTreeMap<NodeId, (QC, Signature, RoundTimeoutReason)>>,

    sig_verifier: SignatureVerifier,
    signer: Signer,
    // ordered_nodes_tx: UnboundedSender<OrderedBlocks>,
    failure_tracker: Option<Arc<dyn TRaptrFailureTracker>>,
}

impl<DL: DisseminationLayer> RaptrNode<DL> {
    pub fn new(
        id: NodeId,
        config: Config,
        dissemination: DL,
        detailed_logging: bool,
        metrics: Metrics,
        signer: Signer,
        sig_verifier: SignatureVerifier,
        // ordered_nodes_tx: UnboundedSender<OrderedBlocks>,
        failure_tracker: Option<Arc<dyn TRaptrFailureTracker>>,
    ) -> Self {
        let quorum = config.quorum();
        assert!(config.block_fetch_multiplicity <= quorum);

        RaptrNode {
            node_id: id,
            config: config.clone(),
            dissemination,
            first_committed_block_timestamp: None,
            detailed_logging,
            metrics,
            block_create_time: Default::default(),
            r_ready: 0,
            entry_reason: RoundEntryReason::FullPrefixQC(QC::genesis()),
            r_cur: 0,
            last_qc_vote: (0, 0).into(),
            cc_voted: Default::default(),
            r_timeout: 0,
            qc_high: QC::genesis(),
            committed_qc: QC::genesis(),
            known_qcs: Default::default(),
            pending_blocks: Default::default(),
            satisfied_blocks: Default::default(),
            pending_qcs: Default::default(),
            qcs_without_blocks: Default::default(),
            satisfied_qcs: Default::default(),
            qcs_to_commit: Default::default(),
            leader_proposal: Default::default(),
            blocks: Default::default(),
            available_prefix_cache: (0, 0).into(),
            qc_votes: Default::default(),
            full_prefix_votes: Default::default(),
            received_cc_vote: Default::default(),
            cc_votes: Default::default(),
            tc_votes: Default::default(),
            sig_verifier,
            signer,
            // ordered_nodes_tx,
            failure_tracker,
        }
    }

    fn on_new_satisfied_block(&mut self, block_digest: BlockHash) {
        assert!(!self.satisfied_blocks.contains(&block_digest));

        self.satisfied_blocks.insert(block_digest.clone());
        if let Some(pending_qcs) = self.pending_qcs.remove(&block_digest) {
            for pending_qc in pending_qcs {
                self.on_new_satisfied_qc(pending_qc);
            }
        }
    }

    fn on_new_satisfied_qc(&mut self, qc: QC) {
        assert!(
            !self.satisfied_qcs.contains(&qc.id()),
            "QC {:?} already satisfied",
            qc.id()
        );

        self.satisfied_qcs.insert(qc.id());

        if let Some(pending_blocks) = self.pending_blocks.remove(&qc.id()) {
            for pending_block in pending_blocks {
                self.on_new_satisfied_block(pending_block);
            }
        }

        // Two-chain commit rule:
        // If there exists two adjacent certified blocks B and B' in the chain with consecutive
        // round numbers, i.e., B'.r = B.r + 1, the replica commits B and all its ancestors.
        let parent_qc = self.blocks[qc.block_digest()].parent_qc();
        if !parent_qc.is_genesis() && qc.round() == parent_qc.round() + 1 {
            if !self.qcs_to_commit.contains_key(&parent_qc.id()) {
                self.qcs_to_commit.insert(
                    parent_qc.id(),
                    (parent_qc.clone(), CommitReason::TwoChainRule),
                );
            }
        }
    }

    async fn on_new_qc_with_available_block(
        &self,
        qc: QC,
        block: &Block,
        ctx: &mut impl ContextFor<Self>,
    ) {
        ctx.notify(self.dissemination.module_id(), NewQCWithPayload {
            payload: block.payload().clone(),
            qc,
        })
        .await;
    }

    async fn on_new_block(
        &mut self,
        block: &Block,
        ctx: &mut impl ContextFor<Self>,
        is_fetch: bool,
    ) {
        if self.blocks.contains_key(&block.digest) {
            return;
        }

        if is_fetch {
            observe_block(block.data.timestamp_usecs, "FETCHRECEIVED");
        } else {
            observe_block(block.data.timestamp_usecs, "RECEIVED");
        }

        for qc in self
            .qcs_without_blocks
            .remove(&block.digest)
            .unwrap_or_default()
        {
            self.on_new_qc_with_available_block(qc, block, ctx).await;
        }

        self.blocks.insert(block.digest.clone(), block.clone());
        let parent_qc = block.parent_qc();

        if !self.known_qcs.contains(&parent_qc.id()) {
            self.on_new_qc(parent_qc.clone(), ctx).await;
        }

        if self.satisfied_qcs.contains(&parent_qc.id()) {
            self.on_new_satisfied_block(block.digest.clone());
        } else {
            self.pending_blocks[parent_qc.id()].push(block.digest.clone());
        }
    }

    async fn on_new_qc(&mut self, new_qc: QC, ctx: &mut impl ContextFor<Self>) {
        if self.known_qcs.contains(&new_qc.id()) {
            return;
        }

        // Update `qc_high`
        if new_qc.id() > self.qc_high.id() {
            self.qc_high = new_qc.clone();
        }

        // If not yet CC-voted in new_qc.round() and new_qc.round() > r_timeout, multicast a CC-vote.
        if self.config.enable_commit_votes {
            if !self.cc_voted[new_qc.round()] && new_qc.round() > self.r_timeout {
                self.cc_voted[new_qc.round()] = true;

                let signature = self
                    .signer
                    .sign_tagged(
                        &CcVoteSignatureCommonData {
                            round: new_qc.round(),
                            block_digest: *new_qc.block_digest(),
                        },
                        new_qc.prefix(),
                    )
                    .unwrap();

                self.log_detail(format!("CC-voting for QC {:?}", new_qc.id()));

                if let Some(block) = self.blocks.get(new_qc.block_digest()) {
                    observe_block(block.data.timestamp_usecs, "CCVote");
                }
                ctx.multicast(Message::CcVote(new_qc.clone(), signature))
                    .await;
            }
        }

        if new_qc.is_full() {
            if let Some(block) = self.blocks.get(new_qc.block_digest()) {
                observe_block(block.data.timestamp_usecs, "QCReady");
            }

            // If form or receive a full-prefix qc, advance to the next round after that.
            self.advance_r_ready(
                new_qc.round() + 1,
                RoundEntryReason::FullPrefixQC(new_qc.clone()),
                ctx,
            )
            .await;
        } else {
            // If the QC is not full, advance to the round of the new QC if lagging behind.
            self.advance_r_ready(
                new_qc.round(),
                RoundEntryReason::ThisRoundQC(new_qc.clone()),
                ctx,
            )
            .await;
        }

        self.known_qcs.insert(new_qc.id());

        if let Some(block) = self.blocks.get(new_qc.block_digest()) {
            self.on_new_qc_with_available_block(new_qc.clone(), block, ctx)
                .await;
        } else {
            self.qcs_without_blocks[*new_qc.block_digest()].push(new_qc.clone());
        }

        if self.satisfied_blocks.contains(new_qc.block_digest()) {
            self.on_new_satisfied_qc(new_qc);
        } else {
            if !self.pending_qcs.contains_key(new_qc.block_digest()) {
                ctx.set_timer(
                    Duration::ZERO,
                    TimerEvent::FetchBlock(
                        new_qc.round(),
                        *new_qc.block_digest(),
                        new_qc.signer_ids().collect(),
                    ),
                )
            }

            self.pending_qcs[*new_qc.block_digest()].push(new_qc);
        }
    }

    /// Issues the QC-vote in the current round.
    async fn qc_vote(
        &mut self,
        reason: QcVoteReason,
        prefix: Prefix,
        missing_authors: BitVec,
        ctx: &mut impl ContextFor<Self>,
    ) {
        let round = self.r_cur;

        if reason == QcVoteReason::FullBlock {
            assert_eq!(prefix, N_SUB_BLOCKS);
            assert!(missing_authors.all_zeros());
        }

        if round > self.r_timeout && self.last_qc_vote < (round, prefix).into() {
            self.last_qc_vote = (self.r_cur, prefix).into();

            let block_digest = self.leader_proposal[&round].clone();

            if reason == QcVoteReason::Timer {
                QC_VOTING_PREFIX_HISTOGRAM.observe(prefix as f64);
            }

            let prev_prefix = if self.last_qc_vote.round == round {
                PREFIX_VOTED_PREVIOUSLY_COUNTER.inc();
                Some(self.last_qc_vote.prefix)
            } else {
                None
            };

            // Metrics and logs.
            QC_VOTING_PREFIX_HISTOGRAM.observe(prefix as f64);
            self.log_detail(format!(
                "QC-voting for block {} proposed by node {} with prefix {}/{}, prev prefix {:?} (reason: {:?})",
                round,
                self.config.leader(round),
                prefix,
                N_SUB_BLOCKS,
                prev_prefix,
                reason,
            ));

            let block_timestamp = self.blocks[&block_digest].data.timestamp_usecs;
            match reason {
                QcVoteReason::FullBlock => {
                    if self.last_qc_vote.round == round {
                        PREFIX_VOTED_PREVIOUSLY_COUNTER.inc();
                    }
                    observe_block(block_timestamp, "FullBlockQCVote");
                },
                QcVoteReason::Timer => {
                    if prefix == N_SUB_BLOCKS {
                        QC_TIMER_VOTE_FULLBLOCK_COUNTER.inc();
                    }
                    observe_block(block_timestamp, "TimerQCVote")
                },
            }

            // Sign and multicast the vote.
            let signature = self
                .signer
                .sign_tagged(
                    &QcVoteSignatureCommonData {
                        round,
                        block_digest: block_digest.clone(),
                    },
                    prefix,
                )
                .unwrap();

            ctx.multicast(Message::QcVote(
                round,
                prefix,
                block_digest,
                signature,
                missing_authors,
            ))
            .await;
        }
    }

    async fn advance_r_ready(
        &mut self,
        round: Round,
        reason: RoundEntryReason,
        ctx: &mut impl ContextFor<Self>,
    ) {
        if round > self.r_ready {
            self.r_ready = round;
            self.entry_reason = reason.clone();
            if let Some(failure_tracker) = &self.failure_tracker {
                failure_tracker.push_reason(reason.clone());
            }

            // Upon getting a justification to enter a higher round r, send it to the leader
            // of round r, unless already received a proposal or a QC in round that round.
            // NB: consider broadcasting to all the nodes instead.
            if !self.leader_proposal.contains_key(&round) && self.qc_high.round() < round {
                ctx.unicast(
                    Message::AdvanceRound(round, reason),
                    self.config.leader(round),
                )
                .await;
            }
        }
    }

    async fn available_prefix(&mut self) -> (Prefix, BitVec) {
        assert!(self.leader_proposal.contains_key(&self.r_cur));

        let block_digest = &self.leader_proposal[&self.r_cur];
        let block = &self.blocks[block_digest];

        if self.available_prefix_cache.round != self.r_cur {
            self.available_prefix_cache = (self.r_cur, 0).into();
        }

        let (prefix, missing_authors) = self
            .dissemination
            .available_prefix(&block.payload(), self.available_prefix_cache.prefix)
            .await;

        self.available_prefix_cache.prefix = prefix;

        (prefix, missing_authors)
    }

    async fn commit_qc(&mut self, qc: QC, commit_reason: CommitReason) {
        let ts = self.blocks[qc.block_digest()].data.timestamp_usecs;
        let is_first_commit = self.first_committed_block_timestamp.is_none();

        let voters: BitVec = qc.signer_ids().map(|id| id as u8).collect();
        let payloads = self.commit_qc_impl(qc, commit_reason);

        if is_first_commit {
            if let Some(ts) = self.first_committed_block_timestamp {
                // Notify the dissemination layer about the first committed block timestamp.
                // Used as the common base timestamp by all nodes for logging.
                self.dissemination
                    .set_first_committed_block_timestamp(ts)
                    .await;
            }
        }

        self.dissemination
            .notify_commit(payloads, ts, Some(voters))
            .await;
    }

    fn commit_qc_impl(&mut self, qc: QC, commit_reason: CommitReason) -> Vec<Payload> {
        if qc.id() <= self.committed_qc.id() {
            return vec![];
        }

        let block = &self.blocks[qc.block_digest()];
        let parent = block.parent_qc().clone();

        if parent.is_genesis() {
            self.log_detail(format!("First committed block: {}", qc.round()));
            self.first_committed_block_timestamp =
                Some(UNIX_EPOCH + Duration::from_micros(block.data.timestamp_usecs));
        }

        // Check for safety violations:
        if qc.round() > self.committed_qc.round() && parent.round() < self.committed_qc.round() {
            panic!("Safety violation: committed block was rolled back");
        }
        if parent.round() == self.committed_qc.round()
            && parent.prefix() < self.committed_qc.prefix()
        {
            panic!("Safety violation: optimistically committed transactions were rolled back");
        }

        // First commit the parent block.
        let mut res = self.commit_qc_impl(parent, CommitReason::Indirect);

        // Then, commit the transactions of this block.
        let block = &self.blocks[qc.block_digest()];

        if qc.round() == self.committed_qc.round() {
            // Extending the prefix of an already committed block.

            assert!(qc.prefix() > self.committed_qc.prefix());

            self.log_detail(format!(
                "Extending the prefix of committed block {}: {} -> {} / {}{} ({:?})",
                qc.round(),
                self.committed_qc.prefix(),
                qc.prefix(),
                N_SUB_BLOCKS,
                if qc.is_full() { " (full)" } else { "" },
                commit_reason,
            ));

            res.push(
                block
                    .payload()
                    .take_sub_blocks(self.committed_qc.prefix()..qc.prefix()),
            );

            // Record the metrics
            let now = Instant::now();
            if self.config.leader(qc.round()) == self.node_id {
                for _ in 0..(qc.prefix() - self.committed_qc.prefix()) {
                    RAIKOU_BATCH_CONSENSUS_LATENCY.observe(
                        now.saturating_duration_since(self.block_create_time[&qc.round()])
                            .as_secs_f64(),
                    );
                    self.metrics.batch_consensus_latency.push((
                        now,
                        self.to_deltas(now - self.block_create_time[&qc.round()]),
                    ));
                }
            }
        } else {
            // Committing a new block.

            self.log_detail(format!(
                "Committing block {} proposed by node {} with {} ACs \
                and prefix {}/{} [{}/{} batches]{} ({:?}).",
                qc.round(),
                self.config.leader(qc.round()),
                block.poas().len(),
                qc.prefix(),
                N_SUB_BLOCKS,
                block
                    .sub_blocks()
                    .take(qc.prefix())
                    .map(|b| b.len())
                    .sum::<usize>(),
                block.sub_blocks().map(|b| b.len()).sum::<usize>(),
                if qc.is_full() { " (full)" } else { "" },
                commit_reason,
            ));

            res.push(block.payload().with_prefix(qc.prefix()));

            RAIKOU_BLOCK_COMMIT_RATE.inc();

            // Record the metrics
            let now = Instant::now();
            if self.config.leader(qc.round()) == self.node_id {
                RAIKOU_BLOCK_CONSENSUS_LATENCY.observe(
                    now.saturating_duration_since(self.block_create_time[&qc.round()])
                        .as_secs_f64(),
                );
                observe_block(block.data.timestamp_usecs, "COMMITBLOCK");
                self.metrics.block_consensus_latency.push((
                    now,
                    self.to_deltas(now - self.block_create_time[&qc.round()]),
                ));
                for _ in 0..(block.poas().len() + qc.prefix()) {
                    RAIKOU_BATCH_CONSENSUS_LATENCY.observe(
                        now.saturating_duration_since(self.block_create_time[&qc.round()])
                            .as_secs_f64(),
                    );
                    self.metrics.batch_consensus_latency.push((
                        now,
                        self.to_deltas(now - self.block_create_time[&qc.round()]),
                    ));
                }
            }
        }

        // Finally, update the committed QC variable.
        self.committed_qc = qc;
        res
    }

    fn uncommitted_batches(&self, qc: &QC) -> HashSet<BatchInfo> {
        let mut uncommitted = HashSet::new();

        let mut cur = qc;
        while cur.round() != self.committed_qc.round() {
            if !self.blocks.contains_key(cur.block_digest()) {
                aptos_logger::warn!(
                    "Deduplication failed for QC {:?}. Block from round {} is missing. \
                    This may often happen in an asynchronous network or a \
                    network where the triangle inequality doesn't hold.",
                    cur.id(),
                    cur.round()
                );
                return uncommitted;
            }

            let block = &self.blocks[cur.block_digest()];
            uncommitted.extend(block.poas().iter().map(|ac| ac.info().clone()));
            uncommitted.extend(
                block
                    .sub_blocks()
                    .take(cur.prefix())
                    .flatten()
                    .map(|batch| batch.clone()),
            );
            cur = block.parent_qc();
        }

        if cur.prefix() > self.committed_qc.prefix() {
            let block = &self.blocks[cur.block_digest()];
            uncommitted.extend(
                block
                    .sub_blocks()
                    .take(cur.prefix())
                    .skip(self.committed_qc.prefix())
                    .flatten()
                    .map(|batch| batch.clone()),
            );
        }

        uncommitted
    }

    fn quorum(&self) -> usize {
        self.config.quorum()
    }

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

        aptos_logger::info!("Node {} at {}: Raikou: {}", self.node_id, time_str, msg);
    }

    fn log_detail(&self, msg: String) {
        if self.detailed_logging {
            self.log_info(msg);
        }
    }

    fn compute_timeout_reason(&self, round: Round) -> RoundTimeoutReason {
        if self.last_qc_vote.round == round {
            return RoundTimeoutReason::NoQC;
        }

        match self.leader_proposal.get(&round) {
            None => RoundTimeoutReason::ProposalNotReceived,
            Some(hash) => {
                let payload = self.blocks[hash].payload();
                if let Err(missing_authors) = self.dissemination.check_payload(payload) {
                    RoundTimeoutReason::PayloadUnavailable { missing_authors }
                } else {
                    RoundTimeoutReason::Unknown
                }
            },
        }
    }

    fn aggregate_timeout_reason(
        &self,
        tc_votes: &BTreeMap<NodeId, (QC, Signature, RoundTimeoutReason)>,
    ) -> RoundTimeoutReason {
        let mut reason_voting_power: HashMap<RoundTimeoutReason, usize> = HashMap::new();
        let mut missing_batch_authors: HashMap<usize, usize> = HashMap::new();
        // let ordered_authors = verifier.get_ordered_account_addresses();
        for (_author, (_, _, reason)) in tc_votes {
            // To aggregate the reason, we only care about the variant type itself and
            // exclude any data within the variants.
            let reason_key = match reason {
                reason @ RoundTimeoutReason::Unknown
                | reason @ RoundTimeoutReason::ProposalNotReceived
                | reason @ RoundTimeoutReason::NoQC => reason.clone(),
                RoundTimeoutReason::PayloadUnavailable { missing_authors } => {
                    for missing_idx in missing_authors.iter_ones() {
                        *missing_batch_authors.entry(missing_idx).or_default() += 1;
                    }
                    RoundTimeoutReason::PayloadUnavailable {
                        // Since we care only about the variant type, we replace the bitvec
                        // with a placeholder.
                        missing_authors: BitVec::with_num_bits(self.config.n_nodes as u16),
                    }
                },
            };
            *reason_voting_power.entry(reason_key).or_default() += 1;
        }
        // The aggregated timeout reason is the reason with the most voting power received from
        // at least f+1 peers by voting power. If such voting power does not exist, then the
        // reason is unknown.

        reason_voting_power
            .into_iter()
            .max_by_key(|(_, voting_power)| *voting_power)
            .filter(|(_, voting_power)| *voting_power >= self.quorum())
            .map(|(reason, _)| {
                // If the aggregated reason is due to unavailable payload, we will compute the
                // aggregated missing authors bitvec counting batch authors that have been reported
                // missing by minority peers.
                if matches!(reason, RoundTimeoutReason::PayloadUnavailable { .. }) {
                    let mut aggregated_bitvec = BitVec::with_num_bits(self.config.n_nodes as u16);
                    for (author_idx, voting_power) in missing_batch_authors {
                        if voting_power >= self.quorum() {
                            aggregated_bitvec.set(author_idx as u16);
                        }
                    }
                    RoundTimeoutReason::PayloadUnavailable {
                        missing_authors: aggregated_bitvec,
                    }
                } else {
                    reason
                }
            })
            .unwrap_or(RoundTimeoutReason::Unknown)
    }

    fn aggregate_qc_missing_authors<'a>(
        &self,
        missing_authors_iter: impl Iterator<Item = &'a BitVec>,
    ) -> BitVec {
        let mut agg_missing_authors = BitVec::with_num_bits(self.config.n_nodes as u16);

        for missing_authors in missing_authors_iter {
            agg_missing_authors = agg_missing_authors.bitor(missing_authors);
        }

        agg_missing_authors
    }
}

pub fn duration_since_epoch() -> Duration {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("System time is before the UNIX_EPOCH")
}

/// Record the time during each stage of a block.
pub fn observe_block(timestamp: u64, stage: &'static str) {
    if let Some(t) = duration_since_epoch().checked_sub(Duration::from_micros(timestamp)) {
        BLOCK_TRACING
            .with_label_values(&[stage])
            .observe(t.as_secs_f64());
    }
}

impl<DL: DisseminationLayer> Protocol for RaptrNode<DL> {
    type Message = Message;
    type TimerEvent = TimerEvent;

    protocol! {
        self: self;
        ctx: ctx;

        // Nodes start the protocol by entering round 1.
        upon start {
            let genesis_qc = QC::genesis();
            self.satisfied_blocks.insert(*genesis_qc.block_digest());
            self.satisfied_qcs.insert(genesis_qc.id());
            self.known_qcs.insert(genesis_qc.id());
            self.advance_r_ready(1, RoundEntryReason::FullPrefixQC(QC::genesis()), ctx).await;
        };

        upon [self.r_cur < self.r_ready] {
            let round = self.r_ready;

            self.r_cur = round;

            let leader =  self.config.leader(round);
            self.log_detail(format!("Entering round {} by {:?} and leader {}", round, self.entry_reason, leader));
            ROUND_ENTER_REASON.with_label_values(&[&format!("{}", self.entry_reason)]).inc();

                let timestamp_usecs = duration_since_epoch().as_micros() as u64;
            if self.node_id == leader {
                // Upon entering round r, the leader of round r creates and multicasts a block.
                let reason = self.entry_reason.clone();

                let payload = self
                    .dissemination
                    .prepare_block(
                        self.r_cur,
                        self.uncommitted_batches(reason.qc()),
                        reason.qc().missing_authors().clone(),
                    )
                    .await;

                let block_data = BlockData {
                    timestamp_usecs,
                    payload,
                    reason,
                };

                let digest = block_data.hash();
                let signature = self.signer.sign(&BlockSignatureData { digest }).unwrap();

                let block = Block::new(block_data, signature);

                self.log_detail(format!(
                    "Proposing block {} with {} ACs and {} sub-blocks",
                    round,
                    block.poas().len(),
                    N_SUB_BLOCKS,
                ));

                observe_block(block.data.timestamp_usecs, "Propose");

                self.block_create_time.insert(round, Instant::now());
                ctx.multicast(Message::Propose(block)).await;
            }

            // Upon entering round r, the node starts a timer for leader timeout.
            ctx.set_timer(self.config.leader_timeout, TimerEvent::Timeout(round));
        };

        // Upon receiving a valid block B = [r, parent_qc, cc, tc, acs, batches] from L_r
        // for the first time, if r >= r_cur and r > r_timeout, store the block,
        // execute on_new_qc and advance_round, start a timer for qc-vote,
        // and report missing batches to the leader.
        upon receive [Message::Propose(block)] from [leader] {
            // part of the message verification.
            assert_eq!(self.config.leader(block.round()), leader);

            if !self.leader_proposal.contains_key(&block.round()) {
                self.log_detail(format!(
                    "Received block {} proposed by node {} with {} ACs and {} optimistically proposed batches",
                    block.round(),
                    leader,
                    block.poas().len(),
                    block.sub_blocks().map(|b| b.len()).sum::<usize>(),
                ));

                self.leader_proposal.insert(block.round(), block.digest.clone());
                self.on_new_block(&block, ctx, false).await;

                let round = block.round();
                let BlockData { payload, reason, .. } = block.data;
                self.advance_r_ready(round, reason, ctx).await;

                ctx.notify(
                    self.dissemination.module_id(),
                    ProposalReceived { leader, round, payload },
                ).await;

                if round < self.r_cur {
                    self.log_detail(format!(
                        "Ignoring proposal of block {} by node {} because already in round {}",
                        round,
                        leader,
                        self.r_cur,
                    ));
                } else if round <= self.r_timeout {
                    self.log_detail(format!(
                        "Ignoring proposal of block {} by node {} because already timed out round {}",
                        round,
                        leader,
                        self.r_timeout,
                    ));
                } else {
                    self.log_detail(format!(
                        "Processing proposal of block {} by node {}",
                        round,
                        leader,
                    ));

                    if self.config.enable_partial_qc_votes {
                        ctx.set_timer(self.config.extra_wait_before_qc_vote, TimerEvent::QcVote(round));
                    }
                }
            }
        };

        // A node issues a qc-vote in its current round r_cur up to 2 times:
        // 1. After a timeout after receiving the block,
        //    if not yet voted in this or greater round.
        // 2. When received all optimistically proposed batches.
        //
        // A node only qc-votes if r_cur > r_timeout.

        upon timer [TimerEvent::QcVote(round)] {
            if round == self.r_cur {
                let (prefix, missing_authors) = self.available_prefix().await;
                self.qc_vote(QcVoteReason::Timer, prefix, missing_authors, ctx).await;
            }
        };

        upon event of type [FullBlockAvailable] from [_dissemination_module] {
            upon [FullBlockAvailable { round }] {
                if round == self.r_cur {
                    self.qc_vote(
                        QcVoteReason::FullBlock,
                        N_SUB_BLOCKS,
                        BitVec::with_num_bits(self.config.n_nodes as u16),
                        ctx
                    )
                    .await;
                }
            };
        };

        // A node only processes QC-votes in round r_cur and higher.
        // It can form a QC up to 2 time in a round, when it has received a quorum of QC-votes
        // with the same block digest and either:
        // 1. the node has not yet formed or received any QC for this round or higher; or
        // 2. the node can form the full-prefix QC.
        // Upon forming a QC, execute on_new_qc.
        upon receive [Message::QcVote(round, prefix, block_digest, signature, missing_authors)] from node [p] {
            let vote_is_non_decreasing = self.qc_votes[round][block_digest].get(&p)
                .map(|(old_prefix, _, _)| *old_prefix < prefix)
                .unwrap_or(true);

            if round >= self.r_cur && vote_is_non_decreasing {
                self.qc_votes[round][block_digest].insert(p, (prefix, signature, missing_authors));

                if prefix == N_SUB_BLOCKS {
                    self.full_prefix_votes[round][block_digest.clone()].insert(p);
                }

                let votes = &self.qc_votes[&round][&block_digest];
                let full_prefix_votes = &self.full_prefix_votes[&round][&block_digest];

                if votes.len() >= self.quorum() && (
                    self.qc_high.round() < round ||
                    full_prefix_votes.len() >= self.config.storage_requirement
                ) {
                    let partial_signatures = votes
                        .iter()
                        .map(|(_, (_, signature, _))| signature.clone());

                    let tagged_multi_signature =
                        self.sig_verifier.aggregate_signatures(partial_signatures).unwrap();

                    let vote_prefixes = votes
                        .iter()
                        .map(|(node_id, (prefix, _signature, _))| (*node_id, *prefix))
                        .collect();

                    let missing_authors = self.aggregate_qc_missing_authors(
                        votes.iter().map(|(_, (_, _, missing))| missing)
                    );

                    let qc = QC::new (
                        round,
                        block_digest,
                        vote_prefixes,
                        tagged_multi_signature,
                        missing_authors,
                        self.config.storage_requirement,
                    );

                    QC_PREFIX_HISTOGRAM.observe(prefix as f64);
                    self.log_detail(format!(
                        "Forming a QC for block {} with prefix {}/{}",
                        round,
                        qc.prefix(),
                        N_SUB_BLOCKS,
                    ));

                    if full_prefix_votes.len() >= self.config.storage_requirement {
                        assert!(
                            qc.is_full(),
                            "Invalid QC prefix: should be full, but {}/{}",
                            qc.prefix(),
                            N_SUB_BLOCKS
                        );
                    }

                    self.on_new_qc(qc, ctx).await;
                }
            }
        };

        // Upon receiving a commit vote for a round-r qc from a node for the
        // first time, store it and execute on_new_qc.
        // Upon having gathered a quorum of commit votes, form a CC,
        // commit the smallest prefix, and execute advance_round.
        upon receive [Message::CcVote(qc, signature)] from node [p] {
            let round = qc.round();

            if !self.received_cc_vote[round].contains(&p) {
                if !self.known_qcs.contains(&qc.id()) {
                    self.on_new_qc(qc.clone(), ctx).await;
                }

                self.received_cc_vote[round].insert(p);
                self.cc_votes[round].insert((qc.prefix(), p), (qc, signature));

                // Drop the CC vote with the smallest prefix if the quorum is exceeded.
                if self.cc_votes[round].len() > self.config.quorum() {
                    self.cc_votes[round].pop_first();
                }

                let votes = &self.cc_votes[round];
                if votes.len() == self.config.quorum() {
                    let (_, (committed_qc, _)) = votes.first_key_value().unwrap();

                    // Form a CC each time we can commit something new, possibly several
                    // times for the same round.
                    if committed_qc.id() > self.committed_qc.id() {
                        self.log_detail(format!(
                            "Forming a CC for block {} with prefix {}/{}",
                            committed_qc.round(),
                            committed_qc.prefix(),
                            N_SUB_BLOCKS,
                        ));

                        if !self.qcs_to_commit.contains_key(&committed_qc.id()) {
                            self.qcs_to_commit.insert(
                                committed_qc.id(),
                                (committed_qc.clone(), CommitReason::CC),
                            );
                        }

                        let vote_prefixes = votes
                            .iter()
                            .map(|((prefix, node_id), _)| (*node_id, *prefix))
                            .collect();

                        let partial_signatures = votes
                            .iter()
                            .map(|(_, (_, signature))| signature.clone());

                        let tagged_multi_signature =
                            self.sig_verifier.aggregate_signatures(partial_signatures).unwrap();

                        let cc = CC::new(
                            round,
                            *committed_qc.block_digest(),
                            vote_prefixes,
                            tagged_multi_signature,
                        );
                        if let Some(block) = self.blocks.get(committed_qc.block_digest()) {
                            observe_block(block.data.timestamp_usecs, "CCReady");
                        }

                        let (_, (max_qc, _)) = votes.last_key_value().unwrap();
                        self.advance_r_ready(
                            round + 1,
                            RoundEntryReason::CC(cc, max_qc.clone()),
                            ctx
                        ).await;
                    }
                }
            }
        };

        upon [
            !self.qcs_to_commit.is_empty()
            && self.satisfied_qcs.contains(self.qcs_to_commit.keys().next().unwrap())
        ] {
            let (_, (qc, commit_reason)) = self.qcs_to_commit.pop_first().unwrap();
            self.commit_qc(qc, commit_reason).await;
        };

        // When the timeout expires, multicast a signed timeout message
        // with qc_high attached.
        upon timer [TimerEvent::Timeout(round)] {
            if round == self.r_cur {
                self.r_timeout = round;
                let signature = self.signer.sign(
                    &TcVoteSignatureData {
                        timeout_round: round,
                        qc_high_id: self.qc_high.id(),
                    }
                )
                .unwrap();

                self.log_detail(format!(
                    "TC-voting in round {}. qc_high: {:?}",
                    round,
                    self.qc_high.id(),
                ));
                let reason = self.compute_timeout_reason(round);

                ctx.multicast(Message::TcVote(round, self.qc_high.clone(), signature, reason)).await;
            }
        };

        // Upon receiving a valid timeout message, execute on_new_qc.
        // Upon gathering a quorum of matching timeout messages,
        // form the TC and execute advance_round.
        upon receive [Message::TcVote(round, qc, signature, timeout_reason)] from node [p] {
            self.on_new_qc(qc.clone(), ctx).await;

            if round >= self.r_cur {
                self.tc_votes[round].insert(p, (qc, signature, timeout_reason));

                let votes = &self.tc_votes[round];

                if votes.len() == self.quorum() {
                    self.log_detail(format!("Forming a TC for round {}", round));

                    let signatures = votes.values().map(|(_, signature, _)| signature.clone());
                    let aggregated_signature = self.sig_verifier.aggregate_signatures(signatures).unwrap();
                    let timeout_reason = self.aggregate_timeout_reason(votes);

                    let vote_data = votes
                        .iter()
                        .map(|(node_id, (qc, _, _))| (*node_id, qc.id()))
                        .collect();

                    let tc = TC::new(
                        round,
                        vote_data,
                        aggregated_signature,
                        timeout_reason
                    );

                    let max_qc = votes
                        .values()
                        .map(|(qc, _, _)| qc)
                        .max_by_key(|qc| qc.id())
                        .unwrap()
                        .clone();

                    if let Some(digest) = self.leader_proposal.get(&round) {
                        let block = self.blocks.get(digest).unwrap();
                        observe_block(block.data.timestamp_usecs, "TCAggregate");
                    }

                    self.advance_r_ready(
                        round + 1,
                        RoundEntryReason::TC(tc, max_qc),
                        ctx
                    )
                    .await;
                }
            }
        };

        // Upon receiving an AdvanceRound message, execute on_new_qc and advance_round.
        upon receive [Message::AdvanceRound(round, reason)] from [_any_node] {
            self.on_new_qc(reason.qc().clone(), ctx).await;
            self.advance_r_ready(round, reason, ctx).await;
        };

        // Block fetching

        upon timer event [TimerEvent::FetchBlock(block_round, digest, qc_signers)] {
            if !self.blocks.contains_key(&digest) {
                let sample = qc_signers.choose_multiple(
                    &mut rand::thread_rng(),
                    self.config.block_fetch_multiplicity,
                ).collect_vec();

                self.log_detail(format!(
                    "Fetching block {} ({:#x}) from nodes {:?}",
                    block_round,
                    digest,
                    sample,
                ));

                for node in sample {
                    ctx.unicast(Message::FetchReq(digest.clone()), *node).await;
                }

                ctx.set_timer(
                    self.config.block_fetch_interval,
                    TimerEvent::FetchBlock(block_round, digest, qc_signers),
                );
            }
        };

        upon receive [Message::FetchReq(digest)] from [p] {
            if let Some(block) = self.blocks.get(&digest) {
                self.log_detail(format!(
                    "Sending block {} ({:#x}) to {}",
                    block.round(),
                    digest,
                    p,
                ));

                ctx.unicast(Message::FetchResp(block.clone()), p).await;
            } else {
                aptos_logger::warn!("Received FetchReq for unknown block {:#x}", digest);
            }
        };

        upon receive [Message::FetchResp(block)] from [_any_node] {
            self.on_new_block(&block, ctx, true).await;
        };

        // State sync

        upon start {
            ctx.set_timer(self.config.round_sync_interval, TimerEvent::RoundSync);
        };

        // To tolerate message loss during asynchronous periods, nodes periodically
        // broadcast their current round with the proof that they have a valid reason
        // to enter it and the highest QC they have seen.
        // Additionally, if the node has voted to time out the current round,
        // it repeats the timeout message.
        upon timer event [TimerEvent::RoundSync] {
            ctx.multicast(Message::AdvanceRound(
                self.r_ready,
                self.entry_reason.clone(),
            )).await;

            if self.r_timeout == self.r_cur {
                let signature = self.signer.sign(
                    &TcVoteSignatureData {
                        timeout_round: self.r_timeout,
                        qc_high_id: self.qc_high.id(),
                    }
                )
                .unwrap();

                let reason = self.compute_timeout_reason(self.r_timeout);

                ctx.multicast(Message::TcVote(self.r_timeout, self.qc_high.clone(), signature, reason)).await;
            }

            ctx.set_timer(self.config.round_sync_interval, TimerEvent::RoundSync);
        };

        // Logging and halting

        upon start {
            self.log_detail("Started".to_string());
            ctx.set_timer(self.config.end_of_run - Instant::now(), TimerEvent::EndOfRun);
            ctx.set_timer(self.config.status_interval, TimerEvent::Status);
        };

        upon timer [TimerEvent::EndOfRun] {
            self.log_detail("Halting by end-of-run timer".to_string());
            ctx.notify(self.dissemination.module_id(), Kill()).await;
            ctx.halt();
        };

        upon event of type [Kill] from [_any_module] {
            upon [Kill()] {
                self.log_detail("Halting by Kill event".to_string());
                ctx.notify(self.dissemination.module_id(), Kill()).await;
                ctx.halt();
            };
        };

        upon timer [TimerEvent::Status] {
            self.log_detail(format!(
                "STATUS:\n\
                \tr_cur: {}\n\
                \tr_ready: {}\n\
                \tr_timeout: {}\n\
                \tqc_high: {:?}\n\
                \tcommitted_qc: {:?}\n\
                \tqcs_to_commit.len(): {}\n\
                \tqcs_to_commit.first(): {:?}\n\
                \tqcs_to_commit.last(): {:?}\n\
                \tlast satisfied qc: {:?}\n\
                \tnum of qc votes: {:?}\n\
                \tnum of cc votes: {:?}\n\
                \tnum of tc votes: {:?}\n",
                self.r_cur,
                self.r_ready,
                self.r_timeout,
                self.qc_high.id(),
                self.committed_qc.id(),
                self.qcs_to_commit.len(),
                self.qcs_to_commit.first_key_value().map(|(k, _)| k),
                self.qcs_to_commit.last_key_value().map(|(k, _)| k),
                self.satisfied_qcs.last(),
                self.qc_votes[self.r_cur].values().map(|v| v.len()).collect_vec(),
                self.cc_votes[self.r_cur].len(),
                self.tc_votes[self.r_cur].len(),
            ));
            ctx.set_timer(self.config.status_interval, TimerEvent::Status);
        };
    }
}
