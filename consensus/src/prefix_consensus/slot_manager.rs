// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! SlotManager: main orchestrator for multi-slot prefix consensus (Algorithm 4).
//!
//! Runs one slot at a time: broadcasts proposals, collects them via [`SlotState`],
//! spawns SPC via [`SPCSpawner`], commits blocks in two waves (v_low early,
//! v_high delta later), wraps in [`OrderedBlocks`], sends to execution,
//! updates ranking, and advances.

use crate::{
    payload_client::PayloadClient,
    pipeline::{buffer_manager::OrderedBlocks, pipeline_builder::PipelineBuilder},
    prefix_consensus::counters::{PROPOSAL_WAIT_DURATION, SLOT_DURATION, SLOT_START_TRIGGER},
};
use aptos_consensus_types::{
    common::{Author, Payload, PayloadFilter, Round},
    payload_pull_params::PayloadPullParameters,
    pipelined_block::{PipelineFutures, PipelinedBlock},
    utils::PayloadTxnsSize,
    vote_data::VoteData,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_crypto::HashValue;
use aptos_executor_types::state_compute_result::StateComputeResult;
use aptos_logger::prelude::*;
use aptos_prefix_consensus::{
    PrefixVector, ProposalData, PriorityClassifiable, SubprotocolNetworkSender,
    StrongPrefixConsensusMsg, build_block_for_entry,
    certificates::StrongPCCommit,
    slot_ranking::MultiSlotRankingManager,
    slot_state::SlotState,
    slot_types::{
        EntryFetchRequest, EntryFetchResponse, SPCOutput, SlotConsensusMsg, SlotProposal,
        create_signed_slot_proposal,
    },
};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use aptos_validator_transaction_pool as vtxn_pool;
use futures::{channel::oneshot, SinkExt, StreamExt};
use std::{
    collections::{HashMap, HashSet},
    pin::Pin,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::Sleep;

/// Default 2Δ timeout for proposal collection.
const SLOT_PROPOSAL_TIMEOUT_MS: u64 = 300;

// ============================================================================
// PendingCommit: waiting for missing entry data before building block
// ============================================================================

/// State held when a wave (v_low or v_high delta) has unresolved entry data.
///
/// The SlotManager stores this while waiting for late proposals or fetch responses
/// to resolve all missing entry hashes. Once `missing` is empty, the wave is committed.
enum PendingWave {
    /// Wave 1: waiting for v_low entry data before early commit.
    VLow {
        slot: u64,
        v_low: PrefixVector,
        resolved: HashMap<HashValue, ProposalData>,
        missing: HashSet<HashValue>,
    },
    /// Wave 2: waiting for v_high delta entry data before final commit.
    VHighDelta {
        slot: u64,
        v_high: PrefixVector,   // full v_high (not just delta)
        resolved: HashMap<HashValue, ProposalData>,
        missing: HashSet<HashValue>,
    },
}

impl PendingWave {
    /// Whether this is wave 1 (v_low).
    fn is_v_low(&self) -> bool {
        matches!(self, PendingWave::VLow { .. })
    }

    /// Mutable access to the common fields across both variants.
    fn pending_fields(&mut self) -> (u64, &mut HashSet<HashValue>, &mut HashMap<HashValue, ProposalData>) {
        match self {
            PendingWave::VLow { slot, missing, resolved, .. } => (*slot, missing, resolved),
            PendingWave::VHighDelta { slot, missing, resolved, .. } => (*slot, missing, resolved),
        }
    }

    /// Consume and destructure for `commit_wave`.
    ///
    /// Returns (slot, vector, resolved). For VLow the vector is v_low;
    /// for VHighDelta it is the full v_high (commit_wave skips positions
    /// already committed in wave 1).
    fn into_parts(self) -> (u64, PrefixVector, HashMap<HashValue, ProposalData>) {
        match self {
            PendingWave::VLow { slot, v_low, resolved, .. } => (slot, v_low, resolved),
            PendingWave::VHighDelta { slot, v_high, resolved, .. } => (slot, v_high, resolved),
        }
    }
}

// ============================================================================
// SPCSpawner trait: pluggable SPC creation for production vs. test
// ============================================================================

/// Handles returned by an SPC spawner for communicating with the running SPC task.
pub struct SPCHandles {
    /// Channel for forwarding regular SPC messages (InnerPC votes, fetch) to the SPC task.
    pub msg_tx: aptos_channels::UnboundedSender<(Author, StrongPrefixConsensusMsg)>,
    /// Channel for forwarding priority SPC messages (Proposal, EmptyView, Commit) to the SPC task.
    pub priority_tx: aptos_channels::UnboundedSender<(Author, StrongPrefixConsensusMsg)>,
    /// Channel for receiving SPC output (VLow and VHigh) from the SPC task.
    pub output_rx: tokio::sync::mpsc::UnboundedReceiver<SPCOutput>,
    /// Oneshot to signal the SPC task to shut down (with ack).
    pub close_tx: futures::channel::oneshot::Sender<futures::channel::oneshot::Sender<()>>,
}

/// Trait for spawning SPC instances. Production uses `RealSPCSpawner` (real
/// `DefaultStrongPCManager`), tests use `StubSPCSpawner` (immediate v_high).
pub trait SPCSpawner: Send + Sync {
    fn spawn_spc(
        &self,
        slot: u64,
        input_vector: PrefixVector,
        ranking: Vec<Author>,
    ) -> SPCHandles;
}

// ============================================================================
// RealSPCSpawner: production implementation using DefaultStrongPCManager
// ============================================================================

/// Production SPC spawner that creates a real [`DefaultStrongPCManager`] with
/// network bridge, adapter, and output channel, then spawns it as a tokio task.
///
/// Holds per-epoch state (identity, keys, network client, validators) so that
/// `spawn_spc` only needs per-slot parameters.
pub struct RealSPCSpawner<NC> {
    author: Author,
    epoch: u64,
    private_key: Arc<aptos_crypto::bls12381::PrivateKey>,
    validator_verifier: Arc<ValidatorVerifier>,
    consensus_network_client: crate::network_interface::ConsensusNetworkClient<NC>,
}

impl<NC> RealSPCSpawner<NC> {
    pub fn new(
        author: Author,
        epoch: u64,
        private_key: Arc<aptos_crypto::bls12381::PrivateKey>,
        validator_verifier: Arc<ValidatorVerifier>,
        consensus_network_client: crate::network_interface::ConsensusNetworkClient<NC>,
    ) -> Self {
        Self {
            author,
            epoch,
            private_key,
            validator_verifier,
            consensus_network_client,
        }
    }
}

impl<NC> SPCSpawner for RealSPCSpawner<NC>
where
    NC: aptos_network::application::interface::NetworkClientInterface<
            crate::network_interface::ConsensusMsg,
        > + Send
        + Sync
        + 'static,
{
    fn spawn_spc(
        &self,
        slot: u64,
        input_vector: PrefixVector,
        ranking: Vec<Author>,
    ) -> SPCHandles {
        // Create message channels (aptos_channels for gauge tracking)
        let (spc_tx, spc_rx) = aptos_channels::new_unbounded(
            &crate::counters::OP_COUNTERS.gauge("spc_slot_channel_msgs"),
        );
        let (priority_tx, priority_rx) = aptos_channels::new_unbounded(
            &crate::counters::OP_COUNTERS.gauge("spc_priority_channel_msgs"),
        );

        // Create output channel (SPC → SlotManager)
        let (output_tx, output_rx) = tokio::sync::mpsc::unbounded_channel();

        // Create close channel
        let (close_tx, close_rx) = futures::channel::oneshot::channel();

        // Create network bridge → client → sender adapter
        let bridge = crate::network_interface::StrongConsensusNetworkBridge::new(
            self.consensus_network_client.clone(),
        );
        let network_client =
            aptos_prefix_consensus::StrongPrefixConsensusNetworkClient::new(bridge);
        let network_sender = aptos_prefix_consensus::StrongNetworkSenderAdapter::new(
            self.author,
            network_client,
            spc_tx.clone(),
            self.validator_verifier.clone(),
        )
        .with_priority_sender(priority_tx.clone());

        // Create ValidatorSigner from stored Arc<PrivateKey> (cheap Arc clone).
        let signer = ValidatorSigner::new(self.author, self.private_key.clone());
        let manager = aptos_prefix_consensus::DefaultStrongPCManager::new(
            self.author,
            self.epoch,
            slot,
            ranking,
            input_vector,
            network_sender,
            signer,
            self.validator_verifier.clone(),
            Some(output_tx),
        );

        tokio::spawn(manager.run(spc_rx, priority_rx, close_rx));

        SPCHandles {
            msg_tx: spc_tx,
            priority_tx,
            output_rx,
            close_tx,
        }
    }
}

/// Main orchestrator for multi-slot prefix consensus.
///
/// Runs one slot at a time: broadcasts proposals, collects them via
/// [`SlotState`], spawns SPC, commits blocks in two waves (v_low early,
/// v_high delta later), wraps in [`OrderedBlocks`], sends to execution,
/// updates ranking, and advances.
pub struct SlotManager<NS: SubprotocolNetworkSender<SlotConsensusMsg>, SP: SPCSpawner> {
    // Identity
    author: Author,
    epoch: u64,
    validator_signer: ValidatorSigner,
    validator_verifier: Arc<ValidatorVerifier>,

    // Slot state
    current_slot: u64,
    slot_states: HashMap<u64, SlotState>,
    ranking_manager: MultiSlotRankingManager,

    // Per-slot SPC channels (set by run_spc, cleared by build_and_commit_block).
    // spc_msg_tx and spc_priority_tx are always set and cleared together.
    spc_msg_tx: Option<aptos_channels::UnboundedSender<(Author, StrongPrefixConsensusMsg)>>,
    spc_priority_tx: Option<aptos_channels::UnboundedSender<(Author, StrongPrefixConsensusMsg)>>,
    spc_output_rx: Option<tokio::sync::mpsc::UnboundedReceiver<SPCOutput>>,
    spc_close_tx: Option<futures::channel::oneshot::Sender<futures::channel::oneshot::Sender<()>>>,

    // Buffers for SPC messages that arrive before the SPC task is spawned.
    // Keyed by slot: messages for current_slot (pre-spawn) and future slots are
    // buffered here, then drained into spc_msg_tx/spc_priority_tx when run_spc() is called.
    spc_msg_buffer: HashMap<u64, Vec<(Author, StrongPrefixConsensusMsg)>>,
    spc_priority_buffer: HashMap<u64, Vec<(Author, StrongPrefixConsensusMsg)>>,

    // SPC spawner (production vs. test)
    spc_spawner: SP,

    // Two-wave commit state
    pending_wave: Option<PendingWave>,
    next_round: Round,                          // global sequential round counter (persists across slots)
    v_low_committed_positions: HashSet<usize>,  // ranking positions committed in wave 1 (reset per slot)
    buffered_v_high: Option<(u64, PrefixVector, StrongPCCommit)>, // v_high buffered if it arrives while wave 1 is pending

    // Verifiable ranking state (Phase 12)
    last_commit_proof: Option<StrongPCCommit>,           // proof from most recent completed slot (for next slot's proposals)
    last_spc_initial_ranking: Option<Vec<Author>>,       // ranking used for the SPC that produced last_commit_proof
    current_slot_ranking: Option<Vec<Author>>,            // ranking snapshot for current slot (for v_high exclusion demotion)
    current_slot_commit_proof: Option<StrongPCCommit>,   // commit proof for current slot (set by on_spc_v_high_complete)

    // Execution bridge
    execution_channel: futures::channel::mpsc::UnboundedSender<OrderedBlocks>,
    pipeline_builder: Option<PipelineBuilder>,
    parent_pipeline_futs: Option<PipelineFutures>,

    // Payload
    payload_client: Arc<dyn PayloadClient>,

    // Block chain tracking
    parent_block_info: BlockInfo,

    // Network
    network_sender: NS,

    // Timer
    slot_timer: Option<(u64, Pin<Box<Sleep>>)>,
    proposal_timeout: Duration,

    // Latency tracking (for Grafana metrics)
    slot_start_time: Option<Instant>,
    proposal_wait_start: Option<Instant>,
    spc_start_time: Option<Instant>,
    vlow_received_time: Option<Instant>,
}

impl<NS: SubprotocolNetworkSender<SlotConsensusMsg>, SP: SPCSpawner> SlotManager<NS, SP> {
    pub fn new(
        author: Author,
        epoch: u64,
        validator_signer: ValidatorSigner,
        validator_verifier: Arc<ValidatorVerifier>,
        ranking_manager: MultiSlotRankingManager,
        execution_channel: futures::channel::mpsc::UnboundedSender<OrderedBlocks>,
        payload_client: Arc<dyn PayloadClient>,
        parent_block_info: BlockInfo,
        network_sender: NS,
        spc_spawner: SP,
        pipeline_builder: Option<PipelineBuilder>,
        parent_pipeline_futs: Option<PipelineFutures>,
    ) -> Self {
        Self {
            author,
            epoch,
            validator_signer,
            validator_verifier,
            current_slot: 0,
            slot_states: HashMap::new(),
            ranking_manager,
            spc_msg_tx: None,
            spc_priority_tx: None,
            spc_output_rx: None,
            spc_close_tx: None,
            spc_msg_buffer: HashMap::new(),
            spc_priority_buffer: HashMap::new(),
            spc_spawner,
            pending_wave: None,
            next_round: 1,
            v_low_committed_positions: HashSet::new(),
            buffered_v_high: None,
            last_commit_proof: None,
            last_spc_initial_ranking: None,
            current_slot_ranking: None,
            current_slot_commit_proof: None,
            execution_channel,
            pipeline_builder,
            parent_pipeline_futs,
            payload_client,
            parent_block_info,
            network_sender,
            slot_timer: None,
            proposal_timeout: Duration::from_millis(SLOT_PROPOSAL_TIMEOUT_MS),
            slot_start_time: None,
            proposal_wait_start: None,
            spc_start_time: None,
            vlow_received_time: None,
        }
    }

    /// Main event loop. Consumes self, runs as a tokio task.
    pub async fn start(
        mut self,
        mut message_rx: aptos_channels::UnboundedReceiver<(Author, SlotConsensusMsg)>,
        mut close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
        // Wait for network connectivity before starting the first slot.
        // Without this, proposals and SPC messages are broadcast before peers connect,
        // causing the first slot to get stuck with insufficient votes.
        let expected_peers = self.ranking_manager.validator_count().saturating_sub(1);
        if expected_peers > 0 {
            let max_wait = tokio::time::sleep(Duration::from_secs(30));
            tokio::pin!(max_wait);
            loop {
                let connected = self.network_sender.connected_peers();
                if connected >= expected_peers {
                    info!(
                        epoch = self.epoch,
                        connected = connected,
                        "Network ready, starting slot consensus"
                    );
                    break;
                }
                tokio::select! {
                    biased;
                    close_req = &mut close_rx => {
                        if let Ok(ack_tx) = close_req {
                            let _ = ack_tx.send(());
                        }
                        return;
                    }
                    () = &mut max_wait => {
                        warn!(
                            epoch = self.epoch,
                            connected = connected,
                            expected = expected_peers,
                            "Network wait timeout, proceeding with available peers"
                        );
                        break;
                    }
                    () = tokio::time::sleep(Duration::from_millis(500)) => {
                        debug!(
                            epoch = self.epoch,
                            connected = connected,
                            expected = expected_peers,
                            "Waiting for network peers before starting slot consensus"
                        );
                    }
                }
            }
        }

        self.start_new_slot(1).await;

        loop {
            // Take optional receivers out to avoid borrow conflicts in select!
            let mut timer_opt = self.slot_timer.take();
            let mut spc_rx_opt = self.spc_output_rx.take();

            tokio::select! {
                biased;

                // Close signal (highest priority)
                close_req = &mut close_rx => {
                    // Restore before breaking
                    self.slot_timer = timer_opt;
                    self.spc_output_rx = spc_rx_opt;
                    if let Ok(ack_tx) = close_req {
                        let _ = ack_tx.send(());
                    }
                    break;
                }

                // Slot timer
                _ = async {
                    match &mut timer_opt {
                        Some((_, timer)) => timer.as_mut().await,
                        None => futures::future::pending::<()>().await,
                    }
                } => {
                    let (slot, _) = timer_opt.expect("timer branch only fires when timer exists");
                    // Timer fired — don't restore it
                    self.spc_output_rx = spc_rx_opt;
                    self.on_timer_expired(slot).await;
                }

                // SPC output
                output = async {
                    match &mut spc_rx_opt {
                        Some(rx) => rx.recv().await,
                        None => futures::future::pending().await,
                    }
                } => {
                    self.slot_timer = timer_opt;
                    self.spc_output_rx = spc_rx_opt;
                    match output {
                        Some(SPCOutput::VLow { slot, v_low }) => {
                            self.on_spc_v_low(slot, v_low).await;
                        }
                        Some(SPCOutput::VHigh { slot, v_high, commit_proof }) => {
                            // If wave 1 payloads are still pending, buffer v_high
                            if self.pending_wave.as_ref().is_some_and(|p| p.is_v_low()) {
                                info!(
                                    epoch = self.epoch,
                                    slot = slot,
                                    "Wave 1 still pending, buffering v_high"
                                );
                                self.buffered_v_high = Some((slot, v_high, commit_proof));
                            } else {
                                self.on_spc_v_high_complete(slot, v_high, commit_proof).await;
                            }
                        }
                        None => {
                            error!(
                                epoch = self.epoch,
                                current_slot = self.current_slot,
                                "SPC output channel closed without producing v_high — \
                                 SPC task may have exited prematurely"
                            );
                        }
                    }
                }

                // Incoming messages
                Some((author, msg)) = message_rx.next() => {
                    self.slot_timer = timer_opt;
                    self.spc_output_rx = spc_rx_opt;
                    match msg {
                        SlotConsensusMsg::SlotProposal(p) => {
                            self.process_proposal(author, *p).await;
                        }
                        SlotConsensusMsg::StrongPCMsg { slot, msg, .. } => {
                            self.process_spc_message(author, slot, msg).await;
                        }
                        SlotConsensusMsg::EntryFetchRequest(req) => {
                            self.process_entry_fetch_request(author, req).await;
                        }
                        SlotConsensusMsg::EntryFetchResponse(resp) => {
                            self.process_entry_fetch_response(*resp).await;
                        }
                    }
                }
            }
        }
    }

    // ========================================================================
    // Slot lifecycle
    // ========================================================================

    async fn start_new_slot(&mut self, slot: u64) {
        // Record "total" duration for the previous slot (if any)
        if let Some(start) = self.slot_start_time.take() {
            SLOT_DURATION
                .with_label_values(&["total"])
                .observe(start.elapsed().as_secs_f64());
        }

        let slot_start = Instant::now();
        self.slot_start_time = Some(slot_start);
        self.spc_start_time = None;
        self.vlow_received_time = None;

        self.current_slot = slot;
        // Reset per-slot state (next_round persists across slots)
        self.v_low_committed_positions.clear();
        self.buffered_v_high = None;
        info!(epoch = self.epoch, slot = slot, "Starting new slot");

        // Pull payload from mempool
        let pull_start = Instant::now();
        let (validator_txns, payload) = self.pull_payload().await;
        SLOT_DURATION
            .with_label_values(&["payload_pull"])
            .observe(pull_start.elapsed().as_secs_f64());
        let _ = validator_txns; // validator_txns collected in Phase 7

        // Snapshot current ranking for this slot (used for v_high exclusion demotion in finalize_slot)
        self.current_slot_ranking = Some(self.ranking_manager.current_ranking().to_vec());

        // Create and sign proposal (embed previous slot's commit proof for verifiable ranking)
        debug_assert!(
            slot != 1 || self.last_commit_proof.is_none(),
            "Slot 1 should not have a commit proof from a previous slot"
        );
        let own_proof_hash = self.last_commit_proof.as_ref()
            .map(|p| SlotProposal::compute_commit_proof_hash(p));
        info!(
            epoch = self.epoch,
            slot = slot,
            has_last_commit_proof = self.last_commit_proof.is_some(),
            own_proof_hash = ?own_proof_hash.map(|h| format!("{:.8}", h)),
            "start_new_slot: creating own proposal"
        );
        let now_usecs = aptos_infallible::duration_since_epoch().as_micros() as u64;
        let proposal = match create_signed_slot_proposal(
            slot,
            self.epoch,
            self.author,
            payload,
            &self.validator_signer,
            now_usecs,
            self.last_commit_proof.clone(),
        ) {
            Ok(p) => p,
            Err(e) => {
                error!(epoch = self.epoch, slot = slot, error = ?e, "Failed to sign slot proposal");
                return;
            },
        };

        // Get or create SlotState — preserve pre-buffered future proposals
        let n = self.ranking_manager.validator_count();
        self.slot_states
            .entry(slot)
            .or_insert_with(|| SlotState::new(slot, n));

        // Insert own proposal directly (avoids unnecessary self-verification)
        self.slot_states
            .get_mut(&slot)
            .expect("just inserted")
            .insert_proposal(proposal.clone());

        // Broadcast proposal (self-send arrives in event loop, ProposalBuffer rejects duplicate)
        self.network_sender
            .broadcast(SlotConsensusMsg::SlotProposal(Box::new(proposal)))
            .await;

        self.proposal_wait_start = Some(Instant::now());

        // Start 2Δ timer
        self.slot_timer = Some((slot, Box::pin(tokio::time::sleep(self.proposal_timeout))));

        // Check if all proposals already received (pre-buffered + own = all in single-validator case)
        let all_received = self
            .slot_states
            .get(&slot)
            .map_or(false, |s| s.has_all_proposals());
        if all_received {
            if let Some(wait_start) = self.proposal_wait_start {
                PROPOSAL_WAIT_DURATION
                    .with_label_values(&[&self.epoch.to_string()])
                    .observe(wait_start.elapsed().as_secs_f64());
            }
            SLOT_START_TRIGGER
                .with_label_values(&["all_proposals"])
                .inc();
            self.slot_timer = None;
            self.run_spc(slot).await;
        }
    }

    async fn process_proposal(&mut self, author: Author, proposal: SlotProposal) {
        // Verify epoch
        if proposal.epoch != self.epoch {
            debug!(
                epoch = self.epoch,
                proposal_epoch = proposal.epoch,
                "Dropping proposal with wrong epoch"
            );
            return;
        }

        // Verify signature
        if let Err(e) = proposal.verify(&self.validator_verifier) {
            warn!(
                epoch = self.epoch,
                slot = proposal.slot,
                author = %author,
                error = ?e,
                "Proposal signature verification failed"
            );
            return;
        }

        let slot = proposal.slot;

        // Get or create slot state (preserve any existing state for this slot)
        let n = self.ranking_manager.validator_count();
        self.slot_states
            .entry(slot)
            .or_insert_with(|| SlotState::new(slot, n));

        let slot_state = self.slot_states.get_mut(&slot).expect("just inserted");
        let entry_hash = proposal.entry_hash();
        let has_proof = proposal.prev_commit_proof.is_some();
        let proof_hash = proposal.prev_commit_proof_hash;
        info!(
            epoch = self.epoch,
            slot = slot,
            author = %author,
            entry_hash = %format!("{:.8}", entry_hash),
            payload_hash = %format!("{:.8}", proposal.payload_hash),
            timestamp = proposal.timestamp_usecs,
            has_proof = has_proof,
            proof_hash = ?proof_hash.map(|h| format!("{:.8}", h)),
            phase = ?slot_state.phase(),
            "Processing proposal"
        );
        let entry_data = ProposalData::from_proposal(&proposal);
        slot_state.insert_proposal(proposal);

        // If all proposals received for current slot AND SPC not yet started.
        // The spc_msg_tx check guards against starting SPC twice: on_timer_expired
        // may have already started SPC for this slot before all proposals arrived.
        if slot == self.current_slot
            && slot_state.has_all_proposals()
            && self.spc_msg_tx.is_none()
        {
            if let Some(wait_start) = self.proposal_wait_start {
                PROPOSAL_WAIT_DURATION
                    .with_label_values(&[&self.epoch.to_string()])
                    .observe(wait_start.elapsed().as_secs_f64());
            }
            SLOT_START_TRIGGER
                .with_label_values(&["all_proposals"])
                .inc();
            info!(epoch = self.epoch, slot = slot, "All proposals received, starting SPC");
            self.slot_timer = None;
            self.run_spc(slot).await;
        }

        // Check if this late proposal resolves a pending wave
        self.try_resolve_pending(entry_hash, entry_data)
            .await;
    }

    async fn on_timer_expired(&mut self, slot: u64) {
        if slot != self.current_slot {
            debug!(
                epoch = self.epoch,
                slot = slot,
                current_slot = self.current_slot,
                "Ignoring stale timer"
            );
            return;
        }
        if self.spc_msg_tx.is_some() {
            return; // SPC already running
        }
        SLOT_START_TRIGGER
            .with_label_values(&["timer_expired"])
            .inc();
        info!(
            epoch = self.epoch,
            slot = slot,
            "Timer expired, starting SPC with available proposals"
        );
        self.run_spc(slot).await;
    }

    // ========================================================================
    // SPC: run_spc, on_spc_v_low, on_spc_v_high_complete, commit_wave, process_spc_message
    // ========================================================================

    async fn run_spc(&mut self, slot: u64) {
        // Record proposal_wait: time from after broadcast to SPC start.
        if let Some(wait_start) = self.proposal_wait_start {
            SLOT_DURATION
                .with_label_values(&["proposal_wait"])
                .observe(wait_start.elapsed().as_secs_f64());
        }
        self.spc_start_time = Some(Instant::now());

        let slot_state = self
            .slot_states
            .get_mut(&slot)
            .expect("SlotState must exist before run_spc");
        slot_state.prepare_spc_input(self.ranking_manager.current_ranking());
        let input_vector = slot_state
            .input_vector()
            .expect("input_vector set by prepare_spc_input")
            .clone();

        let non_bot_count = input_vector.iter().filter(|h| **h != HashValue::zero()).count();
        let entry_data_map_size = slot_state.entry_data_map().map(|m| m.len()).unwrap_or(0);
        let input_hash_strs: Vec<String> = input_vector
            .iter()
            .enumerate()
            .filter(|(_, h)| **h != HashValue::zero())
            .map(|(pos, h)| format!("pos{}={:.8}", pos, h))
            .collect();
        info!(
            epoch = self.epoch,
            slot = slot,
            input_len = input_vector.len(),
            non_bot_entries = non_bot_count,
            entry_data_map_size = entry_data_map_size,
            proposals_received = slot_state.proposal_buffer().proposal_count(),
            input_hashes = ?input_hash_strs,
            "Spawning SPC task"
        );

        let handles = self.spc_spawner.spawn_spc(
            slot,
            input_vector,
            self.ranking_manager.current_ranking().to_vec(),
        );

        self.spc_msg_tx = Some(handles.msg_tx);
        self.spc_priority_tx = Some(handles.priority_tx);
        self.spc_output_rx = Some(handles.output_rx);
        self.spc_close_tx = Some(handles.close_tx);

        // Drain priority buffer first (proposals, empty-view, commit).
        if let Some(buffered) = self.spc_priority_buffer.remove(&slot) {
            let count = buffered.len();
            info!(
                epoch = self.epoch,
                slot = slot,
                buffered_count = count,
                "Draining pre-spawn SPC priority message buffer"
            );
            let tx = self.spc_priority_tx.as_mut().expect("just set above");
            for (author, msg) in buffered {
                if let Err(e) = tx.send((author, msg)).await {
                    error!(
                        epoch = self.epoch,
                        slot = slot,
                        error = ?e,
                        "Failed to drain buffered priority SPC message"
                    );
                    break;
                }
            }
        }

        // Then drain regular buffer (inner PC votes, fetch).
        if let Some(buffered) = self.spc_msg_buffer.remove(&slot) {
            let count = buffered.len();
            info!(
                epoch = self.epoch,
                slot = slot,
                buffered_count = count,
                "Draining pre-spawn SPC message buffer"
            );
            let tx = self.spc_msg_tx.as_mut().expect("just set above");
            for (author, msg) in buffered {
                if let Err(e) = tx.send((author, msg)).await {
                    error!(
                        epoch = self.epoch,
                        slot = slot,
                        error = ?e,
                        "Failed to drain buffered SPC message"
                    );
                    break;
                }
            }
        }
    }

    /// Handle v_low from SPC (wave 1 — early commit).
    async fn on_spc_v_low(&mut self, slot: u64, v_low: PrefixVector) {
        // Record spc_to_vlow
        if let Some(spc_start) = self.spc_start_time {
            SLOT_DURATION
                .with_label_values(&["spc_to_vlow"])
                .observe(spc_start.elapsed().as_secs_f64());
        }

        let v_low_non_bot: Vec<String> = v_low
            .iter()
            .enumerate()
            .filter(|(_, h)| **h != HashValue::zero())
            .map(|(pos, h)| format!("pos{}={:.8}", pos, h))
            .collect();
        info!(
            epoch = self.epoch,
            slot = slot,
            v_low_len = v_low.len(),
            non_bot = v_low_non_bot.len(),
            v_low_hashes = ?v_low_non_bot,
            "Resolving v_low entries for wave 1 (early commit)"
        );

        let resolve_start = Instant::now();
        let (resolved, missing) = self
            .slot_states
            .get(&slot)
            .expect("SlotState must exist when SPC produces v_low")
            .resolve_missing_entries(&v_low);
        SLOT_DURATION
            .with_label_values(&["vlow_entry_resolution"])
            .observe(resolve_start.elapsed().as_secs_f64());

        if missing.is_empty() {
            self.commit_wave(slot, &v_low, &resolved, None).await;
            // Start vlow_to_vhigh timer AFTER v_low processing is done,
            // so it measures actual wait for v_high on the critical path.
            self.vlow_received_time = Some(Instant::now());
            // Check if v_high was buffered (shouldn't happen if v_low resolves instantly,
            // but handle it for robustness)
            if let Some((vhigh_slot, v_high, proof)) = self.buffered_v_high.take() {
                self.on_spc_v_high_complete(vhigh_slot, v_high, proof).await;
            }
        } else {
            info!(
                epoch = self.epoch,
                slot = slot,
                missing_count = missing.len(),
                "Wave 1 (v_low): missing entries, broadcasting fetch requests"
            );
            let missing_set: HashSet<HashValue> = missing.iter().cloned().collect();
            self.pending_wave = Some(PendingWave::VLow {
                slot,
                v_low,
                resolved,
                missing: missing_set,
            });
            for hash in &missing {
                self.network_sender
                    .broadcast(SlotConsensusMsg::EntryFetchRequest(
                        EntryFetchRequest {
                            slot,
                            epoch: self.epoch,
                            entry_hash: *hash,
                        },
                    ))
                    .await;
            }
        }
    }

    /// Handle v_high from SPC (wave 2 — delta commit + slot finalization).
    async fn on_spc_v_high_complete(&mut self, slot: u64, v_high: PrefixVector, commit_proof: StrongPCCommit) {
        // Record vlow_to_vhigh
        if let Some(vlow_time) = self.vlow_received_time {
            SLOT_DURATION
                .with_label_values(&["vlow_to_vhigh"])
                .observe(vlow_time.elapsed().as_secs_f64());
        }

        let v_high_non_bot: Vec<String> = v_high
            .iter()
            .enumerate()
            .filter(|(_, h)| **h != HashValue::zero())
            .map(|(pos, h)| format!("pos{}={:.8}", pos, h))
            .collect();
        let committing_view = commit_proof.committing_view().unwrap_or(0);
        info!(
            epoch = self.epoch,
            slot = slot,
            v_high_len = v_high.len(),
            v_high_non_bot_count = v_high_non_bot.len(),
            v_high_hashes = ?v_high_non_bot,
            committed_in_wave1 = self.v_low_committed_positions.len(),
            commit_proof_view = committing_view,
            "Processing v_high for wave 2 (delta commit)"
        );

        // Store commit proof for finalize_slot (verifiable ranking)
        self.current_slot_commit_proof = Some(commit_proof);

        // Build delta vector: zero out positions already committed in wave 1
        let delta_vector: PrefixVector = v_high
            .iter()
            .enumerate()
            .map(|(i, h)| {
                if self.v_low_committed_positions.contains(&i) {
                    HashValue::zero()
                } else {
                    *h
                }
            })
            .collect();

        let has_delta = delta_vector.iter().any(|h| *h != HashValue::zero());

        if !has_delta {
            // v_low == v_high or all delta positions are bot. Finalize immediately.
            let v_low_positions: Vec<usize> = self.v_low_committed_positions.iter().cloned().collect();
            info!(
                epoch = self.epoch,
                slot = slot,
                v_high_non_bot = v_high_non_bot,
                v_low_committed_positions = ?v_low_positions,
                "No delta entries in v_high, finalizing slot"
            );
            self.finalize_slot(slot, &v_high).await;
            return;
        }

        let resolve_start = Instant::now();
        let (resolved, missing) = self
            .slot_states
            .get(&slot)
            .expect("SlotState must exist when SPC produces v_high")
            .resolve_missing_entries(&delta_vector);
        SLOT_DURATION
            .with_label_values(&["vhigh_entry_resolution"])
            .observe(resolve_start.elapsed().as_secs_f64());

        if missing.is_empty() {
            self.commit_wave(slot, &v_high, &resolved, Some(&v_high))
                .await;
        } else {
            info!(
                epoch = self.epoch,
                slot = slot,
                missing_count = missing.len(),
                "Wave 2 (v_high delta): missing entries, broadcasting fetch requests"
            );
            let missing_set: HashSet<HashValue> = missing.iter().cloned().collect();
            self.pending_wave = Some(PendingWave::VHighDelta {
                slot,
                v_high,
                resolved,
                missing: missing_set,
            });
            for hash in &missing {
                self.network_sender
                    .broadcast(SlotConsensusMsg::EntryFetchRequest(
                        EntryFetchRequest {
                            slot,
                            epoch: self.epoch,
                            entry_hash: *hash,
                        },
                    ))
                    .await;
            }
        }
    }

    /// Build blocks for one wave (v_low or v_high delta), send to execution.
    ///
    /// Iterates `vector` in ranking order, skipping positions in
    /// `v_low_committed_positions` and bot entries. Each non-skipped non-bot
    /// entry becomes its own block with `round = self.next_round++`.
    ///
    /// If `finalize_with_v_high` is `Some(v_high)`, this is the final wave:
    /// ranking is updated, slot state and SPC channels are cleaned up, and
    /// the next slot starts.
    async fn commit_wave(
        &mut self,
        slot: u64,
        vector: &PrefixVector,
        entry_data_map: &HashMap<HashValue, ProposalData>,
        finalize_with_v_high: Option<&PrefixVector>,
    ) {
        let wave_start = Instant::now();
        let is_vhigh_wave = finalize_with_v_high.is_some();
        let ranking = self.ranking_manager.current_ranking();
        let mut blocks: Vec<Arc<PipelinedBlock>> = Vec::new();
        let mut newly_committed_positions: Vec<usize> = Vec::new();

        for (pos, (hash, author)) in vector.iter().zip(ranking.iter()).enumerate() {
            if *hash == HashValue::zero() {
                continue;
            }
            if self.v_low_committed_positions.contains(&pos) {
                continue;
            }

            let entry_data = entry_data_map
                .get(hash)
                .expect("Entry data missing for committed hash — entry resolution bug");

            // Deterministic timestamp from SPC-agreed ProposalData:
            // max(parent_ts + 1, entry_data.timestamp_usecs)
            let parent_ts = self.parent_block_info.timestamp_usecs();
            let timestamp = entry_data.timestamp_usecs
                .max(parent_ts.checked_add(1).expect("timestamp overflow"));

            let round = self.next_round;
            self.next_round += 1;

            // CRITICAL: pass payload_hash (not entry_hash) to build_block_for_entry.
            // BlockData.proposal_hashes stores "Payload hashes of committed proposals".
            let block = build_block_for_entry(
                self.epoch,
                round,
                timestamp,
                *author,
                entry_data.payload_hash,
                entry_data.payload.clone(),
                self.parent_block_info.id(),
                vec![], // validator_txns
            );

            let pipelined = Arc::new(PipelinedBlock::new(
                block,
                vec![],
                StateComputeResult::new_dummy(),
            ));

            // Set up execution pipeline futures
            if let Some(pipeline_builder) = &self.pipeline_builder {
                if let Some(parent_futs) = self.parent_pipeline_futs.take() {
                    pipeline_builder.build_for_consensus(
                        &pipelined,
                        parent_futs,
                        Box::new(|_, _| {}),
                    );
                    self.parent_pipeline_futs = pipelined.pipeline_futs();
                } else {
                    error!(
                        epoch = self.epoch,
                        slot = slot,
                        round = round,
                        "Missing parent pipeline futs, block execution may fail"
                    );
                }
            }

            // Resolve order_proof_tx so the pipeline's signing future can proceed
            let wrapped_li = WrappedLedgerInfo::new(
                VoteData::dummy(),
                LedgerInfoWithSignatures::new(
                    LedgerInfo::new(BlockInfo::empty(), HashValue::zero()),
                    AggregateSignature::empty(),
                ),
            );
            if let Some(tx) = pipelined.pipeline_tx().lock().as_mut() {
                if let Some(tx) = tx.order_proof_tx.take() {
                    if tx.send(wrapped_li).is_err() {
                        error!(
                            epoch = self.epoch,
                            slot = slot,
                            round = round,
                            "Failed to send order_proof — pipeline receiver dropped"
                        );
                    }
                }
            }

            self.parent_block_info = pipelined.block_info();
            newly_committed_positions.push(pos);
            blocks.push(pipelined);
        }

        // Defensive early-return if no blocks produced
        if blocks.is_empty() {
            info!(
                epoch = self.epoch,
                slot = slot,
                "commit_wave: no blocks produced (all entries bot or already committed)"
            );
            if let Some(v_high) = finalize_with_v_high {
                self.finalize_slot(slot, v_high).await;
            }
            return;
        }

        // Record committed positions (for wave 1 → wave 2 tracking)
        for pos in &newly_committed_positions {
            self.v_low_committed_positions.insert(*pos);
        }

        // Build OrderedBlocks with ordered_proof covering the last block
        let last_block_info = blocks.last().unwrap().block_info();
        let ordered = OrderedBlocks {
            ordered_blocks: blocks,
            ordered_proof: LedgerInfoWithSignatures::new(
                LedgerInfo::new(last_block_info.clone(), HashValue::zero()),
                AggregateSignature::empty(),
            ),
        };

        if let Err(e) = self.execution_channel.unbounded_send(ordered) {
            error!(
                epoch = self.epoch,
                slot = slot,
                error = ?e,
                "Failed to send OrderedBlocks to execution"
            );
        } else {
            info!(
                epoch = self.epoch,
                slot = slot,
                block_count = newly_committed_positions.len(),
                last_round = last_block_info.round(),
                is_final_wave = finalize_with_v_high.is_some(),
                "Wave committed — blocks sent to execution pipeline"
            );
        }

        // Record commit_wave duration (block building + execution send only, excludes finalization)
        let stage = if is_vhigh_wave { "vhigh_commit_wave" } else { "vlow_commit_wave" };
        SLOT_DURATION
            .with_label_values(&[stage])
            .observe(wave_start.elapsed().as_secs_f64());

        if let Some(v_high) = finalize_with_v_high {
            self.finalize_slot(slot, v_high).await;
        }
    }

    /// Finalize the slot: update ranking (SPC-aware demotion), clean up state, advance to next slot.
    ///
    /// The commit proof for SPC-view demotions is extracted from the canonical
    /// proposal (first non-⊥ entry in v_high), NOT from local state. This is
    /// necessary because different validators can commit the same SPC in different
    /// views — using a locally-stored proof would cause ranking divergence.
    async fn finalize_slot(&mut self, slot: u64, v_high: &PrefixVector) {
        let finalize_start = Instant::now();
        let non_bot_count = v_high.iter().filter(|h| **h != HashValue::zero()).count();
        let has_commit_proof = self.current_slot_commit_proof.is_some();
        let has_last_ranking = self.last_spc_initial_ranking.is_some();
        info!(
            epoch = self.epoch,
            slot = slot,
            v_high_len = v_high.len(),
            v_high_non_bot = non_bot_count,
            has_current_slot_commit_proof = has_commit_proof,
            has_last_spc_initial_ranking = has_last_ranking,
            "finalize_slot: starting"
        );

        let current_slot_ranking = self.current_slot_ranking.take()
            .unwrap_or_else(|| self.ranking_manager.current_ranking().to_vec());

        // Sub-stage: extract canonical proof
        let t0 = Instant::now();
        let canonical_proof = self.extract_canonical_proof(slot, v_high);
        SLOT_DURATION
            .with_label_values(&["fin_extract_proof"])
            .observe(t0.elapsed().as_secs_f64());

        let (committing_view, spc_initial_ranking) = if let Some(ref prev_ranking) = self.last_spc_initial_ranking {
            let cv = match &canonical_proof {
                Some(proof) => proof.committing_view().unwrap_or_else(|| {
                    warn!(
                        epoch = self.epoch,
                        slot = slot,
                        "Canonical commit proof has no committing_view (empty QC3 votes), defaulting to 1"
                    );
                    1
                }),
                None => {
                    warn!(
                        epoch = self.epoch,
                        slot = slot,
                        "No canonical proof found in slot proposals despite having last_spc_initial_ranking"
                    );
                    1
                },
            };
            (cv, prev_ranking.clone())
        } else {
            // Slot 1: no previous proof, no SPC-view demotions
            (1, current_slot_ranking.clone())
        };

        // Sub-stage: ranking update
        let t1 = Instant::now();
        self.ranking_manager.update_with_proof(
            committing_view,
            &spc_initial_ranking,
            &current_slot_ranking,
            v_high.len(),
        );
        SLOT_DURATION
            .with_label_values(&["fin_ranking_update"])
            .observe(t1.elapsed().as_secs_f64());

        // Store current slot's commit proof and ranking for the next slot.
        // last_commit_proof: embedded in this validator's own proposals for slot S+1.
        // last_spc_initial_ranking: needed for SPC-view demotions when finalize_slot(S+1)
        // extracts the canonical proof from slot S+1's proposals.
        let storing_proof = self.current_slot_commit_proof.is_some();
        if let Some(proof) = self.current_slot_commit_proof.take() {
            self.last_commit_proof = Some(proof);
            self.last_spc_initial_ranking = Some(current_slot_ranking);
        }
        info!(
            epoch = self.epoch,
            slot = slot,
            stored_commit_proof = storing_proof,
            has_last_commit_proof = self.last_commit_proof.is_some(),
            has_last_spc_initial_ranking = self.last_spc_initial_ranking.is_some(),
            committing_view = committing_view,
            "finalize_slot: ranking updated, proof state for next slot"
        );

        // Sub-stage: cleanup (drop slot state, SPC channels, buffers)
        let t2 = Instant::now();
        self.spc_msg_tx.take();
        self.spc_priority_tx.take();
        self.spc_output_rx.take();
        self.spc_close_tx.take();
        self.spc_msg_buffer.remove(&slot);
        self.spc_priority_buffer.remove(&slot);
        self.pending_wave = None;
        self.buffered_v_high = None;
        self.slot_states.remove(&slot);
        SLOT_DURATION
            .with_label_values(&["fin_cleanup"])
            .observe(t2.elapsed().as_secs_f64());

        info!(
            epoch = self.epoch,
            completed_slot = slot,
            next_slot = slot + 1,
            next_round = self.next_round,
            "Slot finalized, advancing to next slot"
        );

        SLOT_DURATION
            .with_label_values(&["finalization"])
            .observe(finalize_start.elapsed().as_secs_f64());

        self.start_new_slot(slot + 1).await;
    }

    /// Extract the canonical commit proof from the first non-⊥ entry in v_high.
    ///
    /// Looks up each non-⊥ entry_hash in the slot state's `entry_data_map` and
    /// returns the first match's `prev_commit_proof`. Since v_high entries are
    /// SPC-agreed composite hashes pinning down payload + timestamp + proof,
    /// this is deterministic across all honest validators.
    ///
    /// Returns `None` for slot 1 (no predecessor) or if no non-⊥ entry has a proof.
    fn extract_canonical_proof(
        &self,
        slot: u64,
        v_high: &PrefixVector,
    ) -> Option<StrongPCCommit> {
        if slot <= 1 {
            return None;
        }

        let slot_state = match self.slot_states.get(&slot) {
            Some(s) => s,
            None => {
                warn!(
                    epoch = self.epoch,
                    slot = slot,
                    "extract_canonical_proof: slot_state missing for slot"
                );
                return None;
            },
        };

        let non_bot_hashes: Vec<(usize, &HashValue)> = v_high
            .iter()
            .enumerate()
            .filter(|(_, h)| **h != HashValue::zero())
            .collect();

        let entry_data_map_size = slot_state
            .entry_data_map()
            .map(|m| m.len())
            .unwrap_or(0);

        if non_bot_hashes.is_empty() {
            // All-bot v_high: no non-bot entries to look up, no proof to extract.
            // This is expected (not an error) — it just means no proposals were committed.
            info!(
                epoch = self.epoch,
                slot = slot,
                v_high_len = v_high.len(),
                entry_data_map_size = entry_data_map_size,
                "extract_canonical_proof: v_high is all-bot, no proof to extract"
            );
            return None;
        }

        // Find the first non-⊥ entry in v_high whose proof is valid.
        // TODO: Add real proof.verify(&self.validator_verifier) here once we move
        // verification off the event loop (e.g., spawn on blocking thread or batch
        // BLS checks). For now we accept the first proof found — the proposer's BLS
        // signature binds them to prev_commit_proof_hash, so a forged proof would
        // require breaking BLS. This dummy check validates the theory that moving
        // O(n³) proof verification off the hot path fixes SPC throughput.
        for (pos, hash) in &non_bot_hashes {
            if let Some(data) = slot_state.lookup_entry_data(hash) {
                if let Some(ref proof) = data.prev_commit_proof {
                    info!(
                        epoch = self.epoch,
                        slot = slot,
                        position = pos,
                        proof_slot = proof.slot,
                        "extract_canonical_proof: accepting proof (TODO: add crypto verification)"
                    );
                    return data.prev_commit_proof;
                }
            }
        }

        // Log detailed mismatch info for debugging
        let entry_data_map_keys: Vec<String> = slot_state
            .entry_data_map()
            .map(|m| m.keys().map(|k| format!("{:.8}", k)).collect())
            .unwrap_or_default();

        let non_bot_hash_strs: Vec<String> = non_bot_hashes
            .iter()
            .map(|(pos, h)| format!("pos{}={:.8}", pos, h))
            .collect();

        warn!(
            epoch = self.epoch,
            slot = slot,
            non_bot_count = non_bot_hashes.len(),
            entry_data_map_size = entry_data_map_size,
            non_bot_hashes = ?non_bot_hash_strs,
            entry_data_keys = ?entry_data_map_keys,
            "extract_canonical_proof: no entry data found for any non-bot v_high entry"
        );
        None
    }

    async fn process_spc_message(
        &mut self,
        author: Author,
        slot: u64,
        msg: StrongPrefixConsensusMsg,
    ) {
        if slot < self.current_slot {
            debug!(
                epoch = self.epoch,
                slot = slot,
                current_slot = self.current_slot,
                "Dropping SPC message for past slot"
            );
            return;
        }

        // If SPC is running for this slot, forward to the appropriate channel.
        if slot == self.current_slot {
            if msg.is_priority() {
                if let Some(tx) = &mut self.spc_priority_tx {
                    if let Err(e) = tx.send((author, msg)).await {
                        error!(
                            epoch = self.epoch,
                            slot = slot,
                            error = ?e,
                            "Failed to send priority SPC message — SPC task receiver dropped."
                        );
                    }
                    return;
                }
            } else {
                if let Some(tx) = &mut self.spc_msg_tx {
                    if let Err(e) = tx.send((author, msg)).await {
                        error!(
                            epoch = self.epoch,
                            slot = slot,
                            error = ?e,
                            "Failed to send SPC message — SPC task receiver dropped. \
                             SPC may be stalled."
                        );
                    }
                    return;
                }
            }
        }

        // SPC not yet spawned for this slot (current pre-spawn or future slot).
        // Buffer the message so it can be drained when run_spc() is called.
        info!(
            epoch = self.epoch,
            slot = slot,
            current_slot = self.current_slot,
            msg_type = msg.name(),
            "Buffering SPC message (SPC not yet spawned for slot)"
        );
        if msg.is_priority() {
            self.spc_priority_buffer
                .entry(slot)
                .or_default()
                .push((author, msg));
        } else {
            self.spc_msg_buffer
                .entry(slot)
                .or_default()
                .push((author, msg));
        }
    }

    // ========================================================================
    // Entry fetch: request handling, response handling, resolution
    // ========================================================================

    async fn process_entry_fetch_request(&mut self, requester: Author, req: EntryFetchRequest) {
        if req.epoch != self.epoch {
            return;
        }

        // Look up the requested entry data in the slot state (primary + late buffer)
        let data = self
            .slot_states
            .get(&req.slot)
            .and_then(|s| s.lookup_entry_data(&req.entry_hash));

        if let Some(data) = data {
            self.network_sender
                .send_to(
                    requester,
                    SlotConsensusMsg::EntryFetchResponse(Box::new(EntryFetchResponse {
                        slot: req.slot,
                        epoch: req.epoch,
                        entry_hash: req.entry_hash,
                        data,
                    })),
                )
                .await;
        }
    }

    async fn process_entry_fetch_response(&mut self, resp: EntryFetchResponse) {
        if resp.epoch != self.epoch {
            return;
        }

        // Verify entry data integrity: recompute entry_hash from the carried data
        if !resp.verify_entry_hash() {
            warn!(
                epoch = self.epoch,
                slot = resp.slot,
                "Entry fetch response hash mismatch, dropping"
            );
            return;
        }

        // Store the fetched entry data in the slot state so it's available
        // for extract_canonical_proof and future lookups.
        if let Some(slot_state) = self.slot_states.get_mut(&resp.slot) {
            if let Some(map) = slot_state.entry_data_map_mut() {
                map.entry(resp.entry_hash)
                    .or_insert_with(|| resp.data.clone());
            }
        }

        // Check if this resolves a pending wave
        let should_commit = if let Some(pending) = &mut self.pending_wave {
            let (pending_slot, missing, resolved) = pending.pending_fields();
            if pending_slot == resp.slot && missing.remove(&resp.entry_hash) {
                resolved.insert(resp.entry_hash, resp.data);
                missing.is_empty()
            } else {
                false
            }
        } else {
            false
        };

        if should_commit {
            self.commit_resolved_pending_wave().await;
        }
    }

    /// Check if a newly resolved entry_hash resolves a pending wave.
    ///
    /// Called after a late proposal is inserted into the slot state. The entry data
    /// is passed directly to avoid a redundant HashMap lookup + clone.
    async fn try_resolve_pending(&mut self, entry_hash: HashValue, data: ProposalData) {
        let should_commit = if let Some(pending) = &mut self.pending_wave {
            let (_slot, missing, resolved) = pending.pending_fields();
            if missing.remove(&entry_hash) {
                resolved.insert(entry_hash, data);
                missing.is_empty()
            } else {
                false
            }
        } else {
            false
        };

        if should_commit {
            self.commit_resolved_pending_wave().await;
        }
    }

    /// Commit a fully-resolved pending wave. Called when all missing entries
    /// have been received (via fetch response or late proposal).
    async fn commit_resolved_pending_wave(&mut self) {
        let pending = self.pending_wave.take().unwrap();
        let is_wave1 = pending.is_v_low();
        let (slot, vector, resolved) = pending.into_parts();
        // Wave 2 (VHighDelta) finalizes the slot; wave 1 (VLow) does not.
        // For wave 2, vector IS the full v_high.
        let finalize_v_high = if is_wave1 { None } else { Some(&vector) };
        self.commit_wave(slot, &vector, &resolved, finalize_v_high).await;
        if is_wave1 {
            if let Some((vhigh_slot, v_high, proof)) = self.buffered_v_high.take() {
                self.on_spc_v_high_complete(vhigh_slot, v_high, proof).await;
            }
        }
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    async fn pull_payload(&self) -> (Vec<aptos_types::validator_txn::ValidatorTransaction>, Payload) {
        let params = PayloadPullParameters {
            max_poll_time: Duration::from_millis(300),
            max_txns: PayloadTxnsSize::new(1000, 4 * 1024 * 1024),
            max_txns_after_filtering: 1000,
            soft_max_txns_after_filtering: 1000,
            max_inline_txns: PayloadTxnsSize::new(400, 400 * 1024),
            user_txn_filter: PayloadFilter::Empty,
            pending_ordering: false,
            pending_uncommitted_blocks: 0,
            recent_max_fill_fraction: 0.0,
            block_timestamp: aptos_infallible::duration_since_epoch(),
            maybe_optqs_payload_pull_params: None,
        };
        match self
            .payload_client
            .pull_payload(params, vtxn_pool::TransactionFilter::no_op())
            .await
        {
            Ok(result) => result,
            Err(e) => {
                warn!(
                    epoch = self.epoch,
                    error = ?e,
                    "Failed to pull payload from mempool, using empty payload"
                );
                (vec![], Payload::DirectMempool(vec![]))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_consensus_types::common::Payload;
    use aptos_prefix_consensus::{
        QC3,
        slot_types::{create_signed_slot_proposal, SlotProposal},
    };
    use aptos_types::{
        validator_signer::ValidatorSigner,
        validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
    };
    use futures::channel::mpsc as futures_mpsc;
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    /// Create a dummy StrongPCCommit for testing.
    /// Uses an empty QC3 (no votes), epoch=0, slot=given.
    /// NOT suitable for verification — only for passing through channels.
    fn dummy_commit_proof(slot: u64, v_high: &PrefixVector) -> StrongPCCommit {
        StrongPCCommit::new(
            QC3::new(vec![]),     // empty QC3 — not verifiable
            vec![],               // empty certificate chain
            v_high.clone(),
            0,                    // epoch
            slot,
        )
    }

    // ========================================================================
    // Test infrastructure
    // ========================================================================

    /// Stub SPC spawner for tests: immediately sends VLow then VHigh.
    /// Both use the input_vector (simulating v_low == v_high, the best case).
    struct StubSPCSpawner;
    impl SPCSpawner for StubSPCSpawner {
        fn spawn_spc(
            &self,
            slot: u64,
            input_vector: PrefixVector,
            _ranking: Vec<Author>,
        ) -> SPCHandles {
            let (msg_tx, _msg_rx) = aptos_channels::new_unbounded_test();
            let (priority_tx, _priority_rx) = aptos_channels::new_unbounded_test();
            let (output_tx, output_rx) = mpsc::unbounded_channel();
            let (close_tx, _close_rx) = futures::channel::oneshot::channel();

            // Send VLow first, then VHigh (both use input_vector for simplicity)
            let _ = output_tx.send(SPCOutput::VLow {
                slot,
                v_low: input_vector.clone(),
            });
            let commit_proof = dummy_commit_proof(slot, &input_vector);
            let _ = output_tx.send(SPCOutput::VHigh {
                slot,
                v_high: input_vector,
                commit_proof,
            });

            SPCHandles {
                msg_tx,
                priority_tx,
                output_rx,
                close_tx,
            }
        }
    }

    /// Mock network sender that records all broadcast messages.
    #[derive(Clone)]
    struct MockSlotNetworkSender {
        sent_messages: Arc<Mutex<Vec<SlotConsensusMsg>>>,
    }

    impl MockSlotNetworkSender {
        fn new() -> Self {
            Self {
                sent_messages: Arc::new(Mutex::new(Vec::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl SubprotocolNetworkSender<SlotConsensusMsg> for MockSlotNetworkSender {
        async fn broadcast(&self, msg: SlotConsensusMsg) {
            self.sent_messages.lock().unwrap().push(msg);
        }

        async fn send_to(&self, _peer: Author, msg: SlotConsensusMsg) {
            self.sent_messages.lock().unwrap().push(msg);
        }
    }

    /// Mock PayloadClient that always returns empty payload.
    struct MockPayloadClient;

    #[async_trait::async_trait]
    impl PayloadClient for MockPayloadClient {
        async fn pull_payload(
            &self,
            _config: PayloadPullParameters,
            _validator_txn_filter: vtxn_pool::TransactionFilter,
        ) -> anyhow::Result<
            (Vec<aptos_types::validator_txn::ValidatorTransaction>, Payload),
            crate::error::QuorumStoreError,
        > {
            Ok((vec![], Payload::DirectMempool(vec![])))
        }
    }

    /// Process all SPC output: handles VLow (wave 1) then VHigh (wave 2), driving
    /// the two-wave commit flow. StubSPCSpawner sends VLow then VHigh.
    ///
    /// Takes `spc_output_rx` out of the manager to avoid borrow conflicts (same
    /// pattern as the main event loop). The rx is not restored since both messages
    /// are consumed and `finalize_slot` cleans up the field anyway.
    async fn process_spc_output(
        manager: &mut SlotManager<MockSlotNetworkSender, StubSPCSpawner>,
    ) {
        let mut rx = manager.spc_output_rx.take().expect("SPC output rx must exist");
        loop {
            match rx.recv().await.expect("SPC output channel closed") {
                SPCOutput::VLow { slot, v_low } => {
                    manager.on_spc_v_low(slot, v_low).await;
                },
                SPCOutput::VHigh { slot, v_high, commit_proof } => {
                    if manager.pending_wave.as_ref().is_some_and(|p| p.is_v_low()) {
                        manager.buffered_v_high = Some((slot, v_high, commit_proof));
                    } else {
                        manager.on_spc_v_high_complete(slot, v_high, commit_proof).await;
                    }
                    break;
                },
            }
        }
    }

    /// Create n validator signers and a matching verifier.
    fn create_validators(n: usize) -> (Vec<ValidatorSigner>, Arc<ValidatorVerifier>) {
        let signers: Vec<_> = (0..n).map(|_| ValidatorSigner::random(None)).collect();
        let infos: Vec<_> = signers
            .iter()
            .map(|s| ValidatorConsensusInfo::new(s.author(), s.public_key(), 1))
            .collect();
        let verifier = Arc::new(ValidatorVerifier::new(infos));
        (signers, verifier)
    }

    /// Insert a proposal directly into the manager's slot state, bypassing
    /// `process_proposal` verification. Use this for slot > 1 proposals in
    /// lifecycle tests where building a verifiable commit proof is unnecessary.
    async fn insert_proposal_unchecked(
        manager: &mut SlotManager<MockSlotNetworkSender, StubSPCSpawner>,
        signer: &ValidatorSigner,
        slot: u64,
        epoch: u64,
    ) {
        let proposal = create_signed_slot_proposal(
            slot, epoch, signer.author(), Payload::DirectMempool(vec![]), signer, 0, None,
        )
        .unwrap();
        let n = manager.ranking_manager.validator_count();
        manager
            .slot_states
            .entry(slot)
            .or_insert_with(|| SlotState::new(slot, n));
        let slot_state = manager.slot_states.get_mut(&slot).expect("just inserted");
        let entry_hash = proposal.entry_hash();
        let entry_data = ProposalData::from_proposal(&proposal);
        slot_state.insert_proposal(proposal);

        // Check if all proposals received for current slot and SPC not yet started
        if slot == manager.current_slot
            && slot_state.has_all_proposals()
            && manager.spc_msg_tx.is_none()
        {
            manager.slot_timer = None;
            manager.run_spc(slot).await;
        }

        // Check if this resolves a pending wave
        manager.try_resolve_pending(entry_hash, entry_data).await;
    }

    /// Build a SlotManager for testing with n validators, using signer[0] as self.
    fn build_test_manager(
        signers: &[ValidatorSigner],
        verifier: Arc<ValidatorVerifier>,
    ) -> (
        SlotManager<MockSlotNetworkSender, StubSPCSpawner>,
        futures_mpsc::UnboundedReceiver<OrderedBlocks>,
        MockSlotNetworkSender,
    ) {

        let (exec_tx, exec_rx) = futures_mpsc::unbounded();
        let authors: Vec<Author> = signers.iter().map(|s| s.author()).collect();
        let network_sender = MockSlotNetworkSender::new();

        let manager = SlotManager::new(
            signers[0].author(),
            1, // epoch
            signers[0].clone(),
            verifier,
            MultiSlotRankingManager::new(authors),
            exec_tx,
            Arc::new(MockPayloadClient),
            BlockInfo::empty(),
            network_sender.clone(),
            StubSPCSpawner,
            None, // pipeline_builder: not needed for unit tests
            None, // parent_pipeline_futs: not needed for unit tests
        );

        (manager, exec_rx, network_sender)
    }

    // ========================================================================
    // Tests
    // ========================================================================

    #[test]
    fn test_slot_manager_new() {
        let (signers, verifier) = create_validators(4);
        let (manager, _exec_rx, _ns) = build_test_manager(&signers, verifier);

        assert_eq!(manager.current_slot, 0);
        assert_eq!(manager.epoch, 1);
        assert_eq!(manager.author, signers[0].author());
        assert!(manager.slot_states.is_empty());
        assert!(manager.spc_msg_tx.is_none());
        assert!(manager.spc_output_rx.is_none());
        assert!(manager.spc_close_tx.is_none());
        assert!(manager.pending_wave.is_none());
        assert!(manager.slot_timer.is_none());
    }

    #[tokio::test]
    async fn test_single_slot_lifecycle() {
        let (signers, verifier) = create_validators(4);
        let (mut manager, mut exec_rx, _ns) = build_test_manager(&signers, verifier.clone());

        // Start slot 1
        manager.start_new_slot(1).await;
        assert_eq!(manager.current_slot, 1);
        assert!(manager.slot_states.contains_key(&1));
        assert!(manager.slot_timer.is_some()); // Timer running (only 1/4 proposals)

        // Feed proposals from other 3 validators
        for signer in &signers[1..] {
            let proposal = create_signed_slot_proposal(
                1,
                1,
                signer.author(),
                Payload::DirectMempool(vec![]),
                signer,
                0, // test timestamp
                None,
            )
            .unwrap();
            manager.process_proposal(signer.author(), proposal).await;
        }

        // All 4 proposals received → SPC should have been triggered
        // StubSPCSpawner sends VLow then VHigh synchronously → available immediately
        if manager.spc_output_rx.is_some() {
            process_spc_output(&mut manager).await;
        }

        // Wave 1: one OrderedBlocks with 4 per-entry blocks (v_low == v_high, all non-bot)
        let ordered = exec_rx.try_next().unwrap().expect("Wave 1 should be on execution channel");
        assert_eq!(ordered.ordered_blocks.len(), 4);
        assert_eq!(ordered.ordered_blocks[0].block().epoch(), 1);
        // Rounds are sequential: 1, 2, 3, 4
        assert_eq!(ordered.ordered_blocks[0].block().round(), 1);
        assert_eq!(ordered.ordered_blocks[3].block().round(), 4);
        // ordered_proof covers the last block
        assert_eq!(ordered.ordered_proof.commit_info().round(), 4);

        // Manager should have advanced to slot 2
        assert_eq!(manager.current_slot, 2);
    }

    #[tokio::test]
    async fn test_all_proposals_received_cancels_timer() {
        // Single validator: own proposal = all proposals → SPC starts immediately, no timer
        let (signers, verifier) = create_validators(1);
        let (mut manager, mut exec_rx, _ns) = build_test_manager(&signers, verifier);

        manager.start_new_slot(1).await;

        // With 1 validator, the own proposal is the only one needed
        // StubSPCSpawner sends VLow then VHigh synchronously → available immediately
        if manager.spc_output_rx.is_some() {
            process_spc_output(&mut manager).await;
        }

        // Wave 1: 1 block (single validator)
        let ordered = exec_rx.try_next().unwrap().expect("Wave 1 should be on execution channel");
        assert_eq!(ordered.ordered_blocks.len(), 1);
        assert_eq!(ordered.ordered_blocks[0].block().round(), 1);
        assert_eq!(manager.current_slot, 2);
    }

    #[tokio::test]
    async fn test_proposal_verification_rejects_wrong_epoch() {
        let (signers, verifier) = create_validators(4);
        let (mut manager, _exec_rx, _ns) = build_test_manager(&signers, verifier);

        manager.start_new_slot(1).await;

        // Create proposal with wrong epoch
        let proposal = create_signed_slot_proposal(
            1,
            99, // wrong epoch
            signers[1].author(),
            Payload::DirectMempool(vec![]),
            &signers[1],
            0,
            None,
        )
        .unwrap();

        manager.process_proposal(signers[1].author(), proposal).await;

        // Should still only have 1 proposal (own)
        let slot_state = manager.slot_states.get(&1).unwrap();
        assert_eq!(slot_state.proposal_buffer().proposal_count(), 1);
    }

    #[tokio::test]
    async fn test_ranking_updates_across_slots() {
        let (signers, verifier) = create_validators(4);
        let (mut manager, mut exec_rx, _ns) = build_test_manager(&signers, verifier.clone());

        let authors: Vec<Author> = signers.iter().map(|s| s.author()).collect();

        // Slot 1: only 2 proposals (from signers[0] and signers[1])
        manager.start_new_slot(1).await;
        let p1 = create_signed_slot_proposal(
            1, 1, signers[1].author(), Payload::DirectMempool(vec![]), &signers[1], 0, None,
        ).unwrap();
        manager.process_proposal(signers[1].author(), p1).await;

        // Fire timer to trigger SPC with partial proposals
        manager.slot_timer = None;
        manager.run_spc(1).await;
        process_spc_output(&mut manager).await;

        // Wave 1: 2 non-bot entries (signers[0] and [1]), signers[2] and [3] are bot
        let ordered = exec_rx.try_next().unwrap().expect("Wave 1");
        assert_eq!(ordered.ordered_blocks.len(), 2);

        // After slot 1: v_high.len() == 4, so ranking_manager.update_with_proof(..., 4) → no demotion
        // (full prefix, all 4 positions present even if some are ⊥)
        assert_eq!(manager.ranking_manager.current_ranking(), &authors);
        assert_eq!(manager.current_slot, 2);
    }

    #[tokio::test]
    async fn test_block_sent_to_execution_channel() {
        let (signers, verifier) = create_validators(2);
        let (mut manager, mut exec_rx, _ns) = build_test_manager(&signers, verifier.clone());

        manager.start_new_slot(1).await;

        // Send the other proposal
        let p = create_signed_slot_proposal(
            1, 1, signers[1].author(), Payload::DirectMempool(vec![]), &signers[1], 0, None,
        ).unwrap();
        manager.process_proposal(signers[1].author(), p).await;

        if manager.spc_output_rx.is_some() {
            process_spc_output(&mut manager).await;
        }

        // Wave 1: 2 per-entry blocks (2 validators, both non-bot)
        let ordered = exec_rx.try_next().unwrap().expect("Wave 1 on exec channel");
        assert_eq!(ordered.ordered_blocks.len(), 2);
        let block = ordered.ordered_blocks[0].block();
        assert_eq!(block.epoch(), 1);
        assert_eq!(block.round(), 1);
        assert_eq!(ordered.ordered_blocks[1].block().round(), 2);
        // ordered_proof covers the last block
        assert_eq!(
            ordered.ordered_proof.commit_info().round(),
            2
        );
    }

    #[tokio::test]
    async fn test_parent_block_info_updated() {
        let (signers, verifier) = create_validators(1);
        let (mut manager, mut exec_rx, _ns) = build_test_manager(&signers, verifier);

        let initial_parent = manager.parent_block_info.clone();

        // Run slot 1 (1 validator → 1 block)
        manager.start_new_slot(1).await;
        if manager.spc_output_rx.is_some() {
            process_spc_output(&mut manager).await;
        }
        let _block1 = exec_rx.try_next().unwrap().unwrap();

        // Parent should have been updated
        assert_ne!(manager.parent_block_info.id(), initial_parent.id());
        let parent_after_slot1 = manager.parent_block_info.clone();

        // Run slot 2 (start_new_slot(2) was called by finalize_slot for slot 1)
        if manager.spc_output_rx.is_some() {
            process_spc_output(&mut manager).await;
        }
        let _block2 = exec_rx.try_next().unwrap().unwrap();

        // Parent should have been updated again
        assert_ne!(manager.parent_block_info.id(), parent_after_slot1.id());
    }

    #[tokio::test]
    async fn test_close_signal_stops_manager() {
        let (signers, verifier) = create_validators(1);
        let (manager, _exec_rx, _ns) = build_test_manager(&signers, verifier);

        let (msg_tx, msg_rx) = aptos_channels::new_unbounded_test();
        let (close_tx, close_rx) = futures::channel::oneshot::channel();
        let (ack_tx, ack_rx) = futures::channel::oneshot::channel();

        let handle = tokio::spawn(manager.start(msg_rx, close_rx));

        // Send close signal
        close_tx.send(ack_tx).unwrap();

        // Should receive ack
        let result = tokio::time::timeout(Duration::from_secs(2), ack_rx).await;
        assert!(result.is_ok(), "Should receive ack within timeout");

        // Manager task should complete
        let result = tokio::time::timeout(Duration::from_secs(2), handle).await;
        assert!(result.is_ok(), "Manager should exit within timeout");

        drop(msg_tx); // Prevent unused warning
    }

    #[tokio::test]
    async fn test_future_proposals_preserved() {
        let (signers, verifier) = create_validators(4);
        let (mut manager, mut exec_rx, _ns) = build_test_manager(&signers, verifier.clone());

        // Before starting slot 2, insert a proposal for slot 2 from signer[1]
        // (bypasses verify() since slot 2 proposals need a commit proof we don't build here)
        insert_proposal_unchecked(&mut manager, &signers[1], 2, 1).await;

        // Should have created a SlotState for slot 2
        assert!(manager.slot_states.contains_key(&2));
        assert_eq!(
            manager.slot_states.get(&2).unwrap().proposal_buffer().proposal_count(),
            1
        );

        // Now run slot 1 quickly (fast-forward with timer skip)
        manager.start_new_slot(1).await;
        manager.slot_timer = None;
        manager.run_spc(1).await;
        process_spc_output(&mut manager).await;
        let _block1 = exec_rx.try_next().unwrap().unwrap();

        // Now at slot 2 — the pre-buffered proposal should still be there
        assert_eq!(manager.current_slot, 2);
        // Slot 2 state should have 2 proposals: own (from start_new_slot) + pre-buffered
        let slot2_state = manager.slot_states.get(&2).unwrap();
        assert_eq!(slot2_state.proposal_buffer().proposal_count(), 2);
    }

    // ========================================================================
    // Phase 4: Two-wave commit flow tests
    // ========================================================================

    /// Set up a SlotManager with all proposals submitted and SPC started for slot 1.
    /// Discards StubSPCSpawner's automatic VLow/VHigh output, leaving the manager
    /// ready for custom v_low/v_high injection via on_spc_v_low/on_spc_v_high_complete.
    ///
    /// Returns the input_vector (containing entry_hashes) for constructing test v_low/v_high.
    async fn setup_slot_with_custom_spc(
        signers: &[ValidatorSigner],
        verifier: Arc<ValidatorVerifier>,
    ) -> (
        SlotManager<MockSlotNetworkSender, StubSPCSpawner>,
        futures_mpsc::UnboundedReceiver<OrderedBlocks>,
        MockSlotNetworkSender,
        PrefixVector, // input_vector with actual entry_hashes
    ) {
        let (mut manager, exec_rx, ns) = build_test_manager(signers, verifier.clone());
        manager.start_new_slot(1).await;

        // Submit proposals from all other validators
        for signer in &signers[1..] {
            let proposal = create_signed_slot_proposal(
                1, 1, signer.author(), Payload::DirectMempool(vec![]), signer, 100, None,
            )
            .unwrap();
            manager.process_proposal(signer.author(), proposal).await;
        }

        // Verify slot state has a frozen entry_data_map (SPC was started)
        let input_vector = manager
            .slot_states
            .get(&1)
            .expect("slot 1 state must exist")
            .input_vector()
            .expect("input_vector must be set after SPC starts")
            .clone();
        assert!(
            manager.slot_states.get(&1).unwrap().entry_data_map().is_some(),
            "entry_data_map must be set after SPC starts"
        );

        // Discard StubSPCSpawner's automatic VLow/VHigh output
        manager.spc_output_rx.take();

        (manager, exec_rx, ns, input_vector)
    }

    #[tokio::test]
    async fn test_two_wave_v_low_partial_v_high_extends() {
        let (signers, verifier) = create_validators(4);
        let (mut manager, mut exec_rx, _ns, iv) =
            setup_slot_with_custom_spc(&signers, verifier).await;

        // Wave 1: v_low has 2 non-bot entries (positions 0, 1)
        manager
            .on_spc_v_low(1, vec![iv[0], iv[1], HashValue::zero(), HashValue::zero()])
            .await;

        let wave1 = exec_rx
            .try_next()
            .unwrap()
            .expect("Wave 1 OrderedBlocks");
        assert_eq!(wave1.ordered_blocks.len(), 2);
        assert_eq!(wave1.ordered_blocks[0].block().round(), 1);
        assert_eq!(wave1.ordered_blocks[1].block().round(), 2);
        assert_eq!(wave1.ordered_proof.commit_info().round(), 2);

        // Wave 2: v_high has 3 non-bot entries; delta = position 2 only
        let v_high = vec![iv[0], iv[1], iv[2], HashValue::zero()];
        let proof = dummy_commit_proof(1, &v_high);
        manager
            .on_spc_v_high_complete(1, v_high, proof)
            .await;

        let wave2 = exec_rx
            .try_next()
            .unwrap()
            .expect("Wave 2 OrderedBlocks");
        assert_eq!(wave2.ordered_blocks.len(), 1);
        assert_eq!(wave2.ordered_blocks[0].block().round(), 3);
        assert_eq!(wave2.ordered_proof.commit_info().round(), 3);

        assert_eq!(manager.current_slot, 2);
        assert_eq!(manager.next_round, 4);
    }

    #[tokio::test]
    async fn test_v_low_all_bot_v_high_has_entries() {
        let (signers, verifier) = create_validators(4);
        let (mut manager, mut exec_rx, _ns, iv) =
            setup_slot_with_custom_spc(&signers, verifier).await;

        // Wave 1: v_low is all-bot → no blocks
        manager.on_spc_v_low(1, vec![HashValue::zero(); 4]).await;
        assert!(
            exec_rx.try_next().is_err(),
            "No wave 1 blocks expected for all-bot v_low"
        );

        // Wave 2: v_high has 2 entries → 2 blocks
        let v_high = vec![iv[0], iv[1], HashValue::zero(), HashValue::zero()];
        let proof = dummy_commit_proof(1, &v_high);
        manager
            .on_spc_v_high_complete(1, v_high, proof)
            .await;

        let wave2 = exec_rx
            .try_next()
            .unwrap()
            .expect("Wave 2 OrderedBlocks");
        assert_eq!(wave2.ordered_blocks.len(), 2);
        assert_eq!(wave2.ordered_blocks[0].block().round(), 1);
        assert_eq!(wave2.ordered_blocks[1].block().round(), 2);

        assert_eq!(manager.current_slot, 2);
    }

    #[tokio::test]
    async fn test_v_low_equals_v_high_no_wave2_blocks() {
        let (signers, verifier) = create_validators(4);
        let (mut manager, mut exec_rx, _ns, iv) =
            setup_slot_with_custom_spc(&signers, verifier).await;

        // v_low == v_high: all 4 entries committed in wave 1
        let v = iv.clone();
        manager.on_spc_v_low(1, v.clone()).await;

        let wave1 = exec_rx.try_next().unwrap().expect("Wave 1");
        assert_eq!(wave1.ordered_blocks.len(), 4);
        assert_eq!(wave1.ordered_blocks[0].block().round(), 1);
        assert_eq!(wave1.ordered_blocks[3].block().round(), 4);

        // v_high == v_low → empty delta → finalize immediately, no wave 2 blocks
        let proof = dummy_commit_proof(1, &v);
        manager.on_spc_v_high_complete(1, v, proof).await;
        assert!(exec_rx.try_next().is_err(), "No wave 2 blocks expected");

        assert_eq!(manager.current_slot, 2);
        assert_eq!(manager.next_round, 5);
    }

    #[tokio::test]
    async fn test_round_monotonicity_across_waves_and_slots() {
        let (signers, verifier) = create_validators(3);
        let (mut manager, mut exec_rx, _ns, iv) =
            setup_slot_with_custom_spc(&signers, verifier.clone()).await;

        // Slot 1: wave 1 = 2 blocks (rounds 1, 2), wave 2 = 1 block (round 3)
        manager
            .on_spc_v_low(1, vec![iv[0], iv[1], HashValue::zero()])
            .await;
        let w1 = exec_rx.try_next().unwrap().unwrap();
        assert_eq!(w1.ordered_blocks[0].block().round(), 1);
        assert_eq!(w1.ordered_blocks[1].block().round(), 2);

        let v_high_1 = vec![iv[0], iv[1], iv[2]];
        let proof_1 = dummy_commit_proof(1, &v_high_1);
        manager.on_spc_v_high_complete(1, v_high_1, proof_1).await;
        let w2 = exec_rx.try_next().unwrap().unwrap();
        assert_eq!(w2.ordered_blocks[0].block().round(), 3);

        // Slot 2 started by finalize_slot
        assert_eq!(manager.current_slot, 2);

        // Submit proposals for slot 2 to trigger SPC
        // (bypasses verify() since slot 2 proposals need a commit proof we don't build here)
        for signer in &signers[1..] {
            insert_proposal_unchecked(&mut manager, signer, 2, 1).await;
        }

        // Get slot 2's input_vector for constructing v_low/v_high
        let iv2 = manager
            .slot_states
            .get(&2)
            .unwrap()
            .input_vector()
            .unwrap()
            .clone();
        manager.spc_output_rx.take(); // discard StubSPCSpawner output

        // Slot 2: wave 1 = 3 blocks → rounds continue at 4, 5, 6
        manager.on_spc_v_low(2, iv2.clone()).await;
        let w3 = exec_rx.try_next().unwrap().unwrap();
        assert_eq!(w3.ordered_blocks[0].block().round(), 4);
        assert_eq!(w3.ordered_blocks[1].block().round(), 5);
        assert_eq!(w3.ordered_blocks[2].block().round(), 6);

        // v_high == v_low → empty delta
        let proof_2 = dummy_commit_proof(2, &iv2);
        manager.on_spc_v_high_complete(2, iv2, proof_2).await;
        assert!(exec_rx.try_next().is_err());

        assert_eq!(manager.current_slot, 3);
        assert_eq!(manager.next_round, 7);
    }

    #[tokio::test]
    async fn test_buffered_v_high_while_wave1_pending() {
        let (signers, verifier) = create_validators(4);
        let (mut manager, mut exec_rx, _ns, iv) =
            setup_slot_with_custom_spc(&signers, verifier).await;

        // Create a ProposalData whose entry_hash is NOT in the slot state's entry_data_map.
        let secret_txn = crate::test_utils::create_signed_transaction(99);
        let secret_payload = Payload::DirectMempool(vec![secret_txn]);
        let secret_payload_hash = SlotProposal::compute_payload_hash(&secret_payload);
        let secret_data = ProposalData {
            payload_hash: secret_payload_hash,
            payload: secret_payload,
            timestamp_usecs: 500,
            prev_commit_proof: None,
            prev_commit_proof_hash: None,
        };
        let secret_entry_hash = secret_data.entry_hash();

        // v_low: position 0 = iv[0] (resolved), position 1 = secret_entry_hash (MISSING)
        manager
            .on_spc_v_low(
                1,
                vec![iv[0], secret_entry_hash, HashValue::zero(), HashValue::zero()],
            )
            .await;

        // Wave 1 should be pending (missing entry data)
        assert!(exec_rx.try_next().is_err(), "Wave 1 should be pending");
        assert!(manager.pending_wave.as_ref().is_some_and(|p| p.is_v_low()));

        // v_high arrives while wave 1 is pending → buffer it
        let buffered_vhigh = vec![iv[0], secret_entry_hash, iv[2], HashValue::zero()];
        let buffered_proof = dummy_commit_proof(1, &buffered_vhigh);
        manager.buffered_v_high = Some((1, buffered_vhigh, buffered_proof));

        // Resolve the missing entry data via fetch response
        manager
            .process_entry_fetch_response(EntryFetchResponse {
                slot: 1,
                epoch: 1,
                entry_hash: secret_entry_hash,
                data: secret_data,
            })
            .await;

        // Wave 1 committed: 2 blocks (positions 0, 1)
        let wave1 = exec_rx.try_next().unwrap().expect("Wave 1");
        assert_eq!(wave1.ordered_blocks.len(), 2);
        assert_eq!(wave1.ordered_blocks[0].block().round(), 1);
        assert_eq!(wave1.ordered_blocks[1].block().round(), 2);

        // Buffered v_high auto-processed: wave 2 delta = position 2 → 1 block
        let wave2 = exec_rx.try_next().unwrap().expect("Wave 2");
        assert_eq!(wave2.ordered_blocks.len(), 1);
        assert_eq!(wave2.ordered_blocks[0].block().round(), 3);

        assert_eq!(manager.current_slot, 2);
        assert_eq!(manager.next_round, 4);
    }

    #[tokio::test]
    async fn test_non_contiguous_v_low_positions() {
        let (signers, verifier) = create_validators(4);
        let (mut manager, mut exec_rx, _ns, iv) =
            setup_slot_with_custom_spc(&signers, verifier).await;

        // v_low commits positions 0 and 2 (skipping 1)
        manager
            .on_spc_v_low(1, vec![iv[0], HashValue::zero(), iv[2], HashValue::zero()])
            .await;

        let wave1 = exec_rx.try_next().unwrap().expect("Wave 1");
        assert_eq!(wave1.ordered_blocks.len(), 2);
        assert_eq!(wave1.ordered_blocks[0].block().round(), 1);
        assert_eq!(wave1.ordered_blocks[1].block().round(), 2);
        // Block authors follow ranking order: signers[0] at pos 0, signers[2] at pos 2
        assert_eq!(
            wave1.ordered_blocks[0].block().author(),
            Some(signers[0].author())
        );
        assert_eq!(
            wave1.ordered_blocks[1].block().author(),
            Some(signers[2].author())
        );

        // v_high has all 4 non-bot; delta = positions 1 and 3
        let v_high = iv.clone();
        let proof = dummy_commit_proof(1, &v_high);
        manager.on_spc_v_high_complete(1, v_high, proof).await;

        let wave2 = exec_rx.try_next().unwrap().expect("Wave 2");
        assert_eq!(wave2.ordered_blocks.len(), 2);
        assert_eq!(wave2.ordered_blocks[0].block().round(), 3);
        assert_eq!(wave2.ordered_blocks[1].block().round(), 4);
        // Delta authors: signers[1] at pos 1, signers[3] at pos 3
        assert_eq!(
            wave2.ordered_blocks[0].block().author(),
            Some(signers[1].author())
        );
        assert_eq!(
            wave2.ordered_blocks[1].block().author(),
            Some(signers[3].author())
        );

        assert_eq!(manager.current_slot, 2);
        assert_eq!(manager.next_round, 5);
    }
}
