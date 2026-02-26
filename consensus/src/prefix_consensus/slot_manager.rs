// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! SlotManager: main orchestrator for multi-slot prefix consensus (Algorithm 4).
//!
//! Runs one slot at a time: broadcasts proposals, collects them via [`SlotState`],
//! spawns SPC via [`SPCSpawner`], builds blocks from v_high, wraps in
//! [`OrderedBlocks`], sends to execution, updates ranking, and advances.

use crate::{
    payload_client::PayloadClient,
    pipeline::buffer_manager::OrderedBlocks,
};
use aptos_consensus_types::{
    common::{Author, Payload, PayloadFilter},
    payload_pull_params::PayloadPullParameters,
    pipelined_block::PipelinedBlock,
    utils::PayloadTxnsSize,
};
use aptos_crypto::HashValue;
use aptos_executor_types::state_compute_result::StateComputeResult;
use aptos_logger::prelude::*;
use aptos_prefix_consensus::{
    PrefixVector, SubprotocolNetworkSender, StrongPrefixConsensusMsg,
    build_block_from_v_high,
    slot_ranking::MultiSlotRankingManager,
    slot_state::SlotState,
    slot_types::{
        PayloadFetchRequest, PayloadFetchResponse, SlotConsensusMsg, SlotProposal,
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
    time::Duration,
};
use tokio::time::Sleep;

/// Default 2Δ timeout for proposal collection.
const SLOT_PROPOSAL_TIMEOUT_MS: u64 = 300;

// ============================================================================
// PendingCommit: waiting for missing payloads before building block
// ============================================================================

/// State held when v_high has been received but some payloads are still missing.
///
/// The SlotManager stores this while waiting for late proposals or fetch responses
/// to resolve all missing hashes. Once `missing` is empty, the block is built.
struct PendingCommit {
    slot: u64,
    v_high: PrefixVector,
    resolved: HashMap<HashValue, Payload>,
    missing: HashSet<HashValue>,
}

// ============================================================================
// SPCSpawner trait: pluggable SPC creation for production vs. test
// ============================================================================

/// Handles returned by an SPC spawner for communicating with the running SPC task.
pub struct SPCHandles {
    /// Channel for forwarding incoming SPC network messages to the SPC task.
    pub msg_tx: aptos_channels::UnboundedSender<(Author, StrongPrefixConsensusMsg)>,
    /// Channel for receiving committed (slot, v_high) from the SPC task.
    pub output_rx: tokio::sync::mpsc::UnboundedReceiver<(u64, PrefixVector)>,
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
        // Create message channel (aptos_channels for gauge tracking)
        let (spc_tx, spc_rx) = aptos_channels::new_unbounded(
            &crate::counters::OP_COUNTERS.gauge("spc_slot_channel_msgs"),
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
        );

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

        tokio::spawn(manager.run(spc_rx, close_rx));

        SPCHandles {
            msg_tx: spc_tx,
            output_rx,
            close_tx,
        }
    }
}

/// Main orchestrator for multi-slot prefix consensus.
///
/// Runs one slot at a time: broadcasts proposals, collects them via
/// [`SlotState`], spawns SPC, builds blocks from v_high, wraps in
/// [`OrderedBlocks`], sends to execution, updates ranking, and advances.
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

    // Per-slot SPC channels (set by run_spc, cleared by on_spc_v_high)
    spc_msg_tx: Option<aptos_channels::UnboundedSender<(Author, StrongPrefixConsensusMsg)>>,
    spc_output_rx: Option<tokio::sync::mpsc::UnboundedReceiver<(u64, PrefixVector)>>,
    spc_close_tx: Option<futures::channel::oneshot::Sender<futures::channel::oneshot::Sender<()>>>,

    // SPC spawner (production vs. test)
    spc_spawner: SP,

    // Pending commit: waiting for missing payloads before block construction
    pending_commit: Option<PendingCommit>,

    // Execution bridge
    execution_channel: futures::channel::mpsc::UnboundedSender<OrderedBlocks>,

    // Payload
    payload_client: Arc<dyn PayloadClient>,

    // Block chain tracking
    parent_block_info: BlockInfo,

    // Network
    network_sender: NS,

    // Timer
    slot_timer: Option<(u64, Pin<Box<Sleep>>)>,
    proposal_timeout: Duration,
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
            spc_output_rx: None,
            spc_close_tx: None,
            spc_spawner,
            pending_commit: None,
            execution_channel,
            payload_client,
            parent_block_info,
            network_sender,
            slot_timer: None,
            proposal_timeout: Duration::from_millis(SLOT_PROPOSAL_TIMEOUT_MS),
        }
    }

    /// Main event loop. Consumes self, runs as a tokio task.
    pub async fn start(
        mut self,
        mut message_rx: aptos_channels::UnboundedReceiver<(Author, SlotConsensusMsg)>,
        mut close_rx: oneshot::Receiver<oneshot::Sender<()>>,
    ) {
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
                    self.on_timer_expired(slot);
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
                    if let Some((slot, v_high)) = output {
                        self.on_spc_v_high(slot, v_high).await;
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
                        SlotConsensusMsg::PayloadFetchRequest(req) => {
                            self.process_payload_fetch_request(author, req).await;
                        }
                        SlotConsensusMsg::PayloadFetchResponse(resp) => {
                            self.process_payload_fetch_response(*resp).await;
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
        self.current_slot = slot;
        info!(epoch = self.epoch, slot = slot, "Starting new slot");

        // Pull payload from mempool
        let (validator_txns, payload) = self.pull_payload().await;
        let _ = validator_txns; // validator_txns collected in Phase 7

        // Create and sign proposal
        let proposal = match create_signed_slot_proposal(
            slot,
            self.epoch,
            self.author,
            payload,
            &self.validator_signer,
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

        // Start 2Δ timer
        self.slot_timer = Some((slot, Box::pin(tokio::time::sleep(self.proposal_timeout))));

        // Check if all proposals already received (pre-buffered + own = all in single-validator case)
        let all_received = self
            .slot_states
            .get(&slot)
            .map_or(false, |s| s.has_all_proposals());
        if all_received {
            self.slot_timer = None;
            self.run_spc(slot);
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
        let proposal_hash = proposal.payload_hash;
        let proposal_payload = proposal.payload.clone();
        slot_state.insert_proposal(proposal);

        // If all proposals received for current slot AND SPC not yet started.
        // The spc_msg_tx check guards against starting SPC twice: on_timer_expired
        // may have already started SPC for this slot before all proposals arrived.
        if slot == self.current_slot
            && slot_state.has_all_proposals()
            && self.spc_msg_tx.is_none()
        {
            info!(epoch = self.epoch, slot = slot, "All proposals received, starting SPC");
            self.slot_timer = None;
            self.run_spc(slot);
        }

        // Check if this late proposal resolves a pending commit
        self.try_resolve_pending(proposal_hash, proposal_payload)
            .await;
    }

    fn on_timer_expired(&mut self, slot: u64) {
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
        info!(
            epoch = self.epoch,
            slot = slot,
            "Timer expired, starting SPC with available proposals"
        );
        self.run_spc(slot);
    }

    // ========================================================================
    // SPC: run_spc, on_spc_v_high, process_spc_message
    // ========================================================================

    fn run_spc(&mut self, slot: u64) {
        let slot_state = self
            .slot_states
            .get_mut(&slot)
            .expect("SlotState must exist before run_spc");
        slot_state.prepare_spc_input(self.ranking_manager.current_ranking());
        let input_vector = slot_state
            .input_vector()
            .expect("input_vector set by prepare_spc_input")
            .clone();

        let handles = self.spc_spawner.spawn_spc(
            slot,
            input_vector,
            self.ranking_manager.current_ranking().to_vec(),
        );

        self.spc_msg_tx = Some(handles.msg_tx);
        self.spc_output_rx = Some(handles.output_rx);
        self.spc_close_tx = Some(handles.close_tx);
    }

    async fn on_spc_v_high(&mut self, slot: u64, v_high: PrefixVector) {
        info!(
            epoch = self.epoch,
            slot = slot,
            v_high_len = v_high.len(),
            "SPC completed, resolving payloads"
        );

        // Resolve payloads from primary payload_map + late proposals
        let (resolved, missing) = self
            .slot_states
            .get(&slot)
            .expect("SlotState must exist when SPC completes")
            .resolve_missing_payloads(&v_high);

        if missing.is_empty() {
            // Happy path: all payloads available
            self.build_and_commit_block(slot, v_high, resolved).await;
        } else {
            info!(
                epoch = self.epoch,
                slot = slot,
                missing_count = missing.len(),
                "Missing payloads, broadcasting fetch requests"
            );

            // Store pending commit
            let missing_set: HashSet<HashValue> = missing.iter().cloned().collect();
            self.pending_commit = Some(PendingCommit {
                slot,
                v_high,
                resolved,
                missing: missing_set,
            });

            // Broadcast fetch requests for each missing hash
            for hash in &missing {
                self.network_sender
                    .broadcast(SlotConsensusMsg::PayloadFetchRequest(
                        PayloadFetchRequest {
                            slot,
                            epoch: self.epoch,
                            payload_hash: *hash,
                        },
                    ))
                    .await;
            }
        }
    }

    /// Build block from v_high and resolved payloads, send to execution, advance slot.
    async fn build_and_commit_block(
        &mut self,
        slot: u64,
        v_high: PrefixVector,
        payload_map: HashMap<HashValue, Payload>,
    ) {
        info!(
            epoch = self.epoch,
            slot = slot,
            "Building block from v_high"
        );

        // Compute timestamp: max(parent + 1, now)
        let parent_ts = self.parent_block_info.timestamp_usecs();
        let now_usecs = aptos_infallible::duration_since_epoch().as_micros() as u64;
        let timestamp = now_usecs.max(parent_ts.checked_add(1).expect("timestamp overflow"));

        // Build block (round == slot)
        let block = build_block_from_v_high(
            self.epoch,
            slot,                                    // round (== slot)
            timestamp,
            self.ranking_manager.current_ranking(),  // ranking
            &v_high,
            &payload_map,
            self.parent_block_info.id(),             // parent_block_id
            vec![],                                  // validator_txns
        );

        // Wrap in PipelinedBlock + OrderedBlocks
        let pipelined = Arc::new(PipelinedBlock::new(
            block,
            vec![],
            StateComputeResult::new_dummy(),
        ));
        let block_info = pipelined.block_info();
        let ordered = OrderedBlocks {
            ordered_blocks: vec![pipelined],
            ordered_proof: LedgerInfoWithSignatures::new(
                LedgerInfo::new(block_info.clone(), HashValue::zero()),
                AggregateSignature::empty(),
            ),
        };

        // Send to execution
        if let Err(e) = self.execution_channel.unbounded_send(ordered) {
            error!(
                epoch = self.epoch,
                slot = slot,
                error = ?e,
                "Failed to send OrderedBlocks to execution"
            );
        }

        // Update parent tracking
        self.parent_block_info = block_info;

        // Update ranking: v_high.len() is the committed prefix length ℓ.
        // If ℓ < n, the validator at position ℓ (first excluded) is demoted.
        self.ranking_manager.update(v_high.len());

        // Clean up slot state, SPC channels, and pending commit
        self.spc_msg_tx.take();
        self.spc_output_rx.take();
        self.spc_close_tx.take();
        self.pending_commit = None;
        self.slot_states.remove(&slot);

        // Advance to next slot
        self.start_new_slot(slot + 1).await;
    }

    async fn process_spc_message(
        &mut self,
        author: Author,
        slot: u64,
        msg: StrongPrefixConsensusMsg,
    ) {
        if slot != self.current_slot {
            debug!(
                epoch = self.epoch,
                slot = slot,
                current_slot = self.current_slot,
                "Dropping SPC message for non-current slot"
            );
            return;
        }
        if let Some(tx) = &mut self.spc_msg_tx {
            let _ = tx.send((author, msg)).await;
        }
    }

    // ========================================================================
    // Payload fetch: request handling, response handling, resolution
    // ========================================================================

    async fn process_payload_fetch_request(&mut self, requester: Author, req: PayloadFetchRequest) {
        if req.epoch != self.epoch {
            return;
        }

        // Look up the requested payload in the slot state (primary + late buffer)
        let payload = self
            .slot_states
            .get(&req.slot)
            .and_then(|s| s.lookup_payload(&req.payload_hash));

        if let Some(payload) = payload {
            self.network_sender
                .send_to(
                    requester,
                    SlotConsensusMsg::PayloadFetchResponse(Box::new(PayloadFetchResponse {
                        slot: req.slot,
                        epoch: req.epoch,
                        payload_hash: req.payload_hash,
                        payload,
                    })),
                )
                .await;
        }
    }

    // TODO(production): A Byzantine node can send unsolicited PayloadFetchResponse
    // messages, forcing us to compute H(payload) for each one. The cost is low
    // (SHA3-256 over serialized payload), but for hardening we should check
    // `pending_commit.missing.contains(&resp.payload_hash)` *before* computing
    // the hash, so unsolicited responses are dropped at near-zero cost.
    async fn process_payload_fetch_response(&mut self, resp: PayloadFetchResponse) {
        if resp.epoch != self.epoch {
            return;
        }

        // Verify payload integrity
        if !resp.verify_payload_hash() {
            warn!(
                epoch = self.epoch,
                slot = resp.slot,
                "Payload fetch response hash mismatch, dropping"
            );
            return;
        }

        // Check if this resolves a pending commit
        if let Some(pending) = &mut self.pending_commit {
            if pending.slot == resp.slot && pending.missing.remove(&resp.payload_hash) {
                pending.resolved.insert(resp.payload_hash, resp.payload);

                if pending.missing.is_empty() {
                    // All payloads resolved — build block
                    let pending = self.pending_commit.take().unwrap();
                    self.build_and_commit_block(pending.slot, pending.v_high, pending.resolved)
                        .await;
                }
            }
        }
    }

    /// Check if a newly resolved payload_hash resolves a pending commit.
    ///
    /// Called after a late proposal is inserted into the slot state. The payload
    /// is passed directly to avoid a redundant HashMap lookup + clone.
    async fn try_resolve_pending(&mut self, payload_hash: HashValue, payload: Payload) {
        let should_commit = if let Some(pending) = &mut self.pending_commit {
            if pending.missing.remove(&payload_hash) {
                pending.resolved.insert(payload_hash, payload);
                pending.missing.is_empty()
            } else {
                false
            }
        } else {
            false
        };

        if should_commit {
            let pending = self.pending_commit.take().unwrap();
            self.build_and_commit_block(pending.slot, pending.v_high, pending.resolved)
                .await;
        }
    }

    // ========================================================================
    // Helpers
    // ========================================================================

    async fn pull_payload(&self) -> (Vec<aptos_types::validator_txn::ValidatorTransaction>, Payload) {
        let params = PayloadPullParameters {
            max_poll_time: Duration::from_millis(300),
            max_txns: PayloadTxnsSize::new(500, 1024 * 1024),
            max_txns_after_filtering: 500,
            soft_max_txns_after_filtering: 500,
            max_inline_txns: PayloadTxnsSize::new(100, 100 * 1024),
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
    use aptos_prefix_consensus::slot_types::create_signed_slot_proposal;
    use aptos_types::{
        validator_signer::ValidatorSigner,
        validator_verifier::{ValidatorConsensusInfo, ValidatorVerifier},
    };
    use futures::channel::mpsc as futures_mpsc;
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    // ========================================================================
    // Test infrastructure
    // ========================================================================

    /// Stub SPC spawner for tests: immediately returns input_vector as v_high.
    struct StubSPCSpawner;

    impl SPCSpawner for StubSPCSpawner {
        fn spawn_spc(
            &self,
            slot: u64,
            input_vector: PrefixVector,
            _ranking: Vec<Author>,
        ) -> SPCHandles {
            let (msg_tx, _msg_rx) = aptos_channels::new_unbounded_test();
            let (output_tx, output_rx) = mpsc::unbounded_channel();
            let (close_tx, _close_rx) = futures::channel::oneshot::channel();

            // Immediately send input_vector as v_high
            let _ = output_tx.send((slot, input_vector));

            SPCHandles {
                msg_tx,
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
        assert!(manager.pending_commit.is_none());
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
            )
            .unwrap();
            manager.process_proposal(signer.author(), proposal).await;
        }

        // All 4 proposals received → SPC should have been triggered
        // StubSPCSpawner sends output synchronously → available immediately
        if manager.spc_output_rx.is_some() {
            let (slot, v_high) = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
            manager.on_spc_v_high(slot, v_high).await;
        }

        // Block should have been sent to execution
        let ordered = exec_rx.try_next().unwrap().expect("Block should be on execution channel");
        assert_eq!(ordered.ordered_blocks.len(), 1);
        let block = ordered.ordered_blocks[0].block();
        assert_eq!(block.epoch(), 1);
        assert_eq!(block.round(), 1); // round == slot

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
        // StubSPCSpawner sends output synchronously → available immediately
        if manager.spc_output_rx.is_some() {
            let (slot, v_high) = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
            manager.on_spc_v_high(slot, v_high).await;
        }

        // Block should be on exec channel
        let ordered = exec_rx.try_next().unwrap().expect("Block should be on execution channel");
        assert_eq!(ordered.ordered_blocks.len(), 1);
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
            1, 1, signers[1].author(), Payload::DirectMempool(vec![]), &signers[1],
        ).unwrap();
        manager.process_proposal(signers[1].author(), p1).await;

        // Fire timer to trigger SPC with partial proposals
        manager.slot_timer = None;
        manager.run_spc(1);
        let (slot, v_high) = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();

        // v_high has length 4 (full ranking), entries for signers[2] and [3] are zero
        assert_eq!(v_high.len(), 4);
        manager.on_spc_v_high(slot, v_high).await;

        // Consume the block
        let _block1 = exec_rx.try_next().unwrap().expect("Block 1");

        // After slot 1: v_high.len() == 4, so ranking_manager.update(4) → no demotion
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
            1, 1, signers[1].author(), Payload::DirectMempool(vec![]), &signers[1],
        ).unwrap();
        manager.process_proposal(signers[1].author(), p).await;

        if manager.spc_output_rx.is_some() {
            let (slot, v_high) = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
            manager.on_spc_v_high(slot, v_high).await;
        }

        let ordered = exec_rx.try_next().unwrap().expect("Block on exec channel");
        assert_eq!(ordered.ordered_blocks.len(), 1);
        let block = ordered.ordered_blocks[0].block();
        assert_eq!(block.epoch(), 1);
        assert_eq!(block.round(), 1);
        // Verify it has a valid proof
        assert_eq!(
            ordered.ordered_proof.commit_info().round(),
            1
        );
    }

    #[tokio::test]
    async fn test_parent_block_info_updated() {
        let (signers, verifier) = create_validators(1);
        let (mut manager, mut exec_rx, _ns) = build_test_manager(&signers, verifier);

        let initial_parent = manager.parent_block_info.clone();

        // Run slot 1
        manager.start_new_slot(1).await;
        if manager.spc_output_rx.is_some() {
            let (slot, v_high) = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
            manager.on_spc_v_high(slot, v_high).await;
        }
        let _block1 = exec_rx.try_next().unwrap().unwrap();

        // Parent should have been updated
        assert_ne!(manager.parent_block_info.id(), initial_parent.id());
        let parent_after_slot1 = manager.parent_block_info.clone();

        // Run slot 2 (start_new_slot(2) was called by on_spc_v_high for slot 1)
        if manager.spc_output_rx.is_some() {
            let (slot, v_high) = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
            manager.on_spc_v_high(slot, v_high).await;
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
        let future_proposal = create_signed_slot_proposal(
            2, 1, signers[1].author(), Payload::DirectMempool(vec![]), &signers[1],
        ).unwrap();
        manager.process_proposal(signers[1].author(), future_proposal).await;

        // Should have created a SlotState for slot 2
        assert!(manager.slot_states.contains_key(&2));
        assert_eq!(
            manager.slot_states.get(&2).unwrap().proposal_buffer().proposal_count(),
            1
        );

        // Now run slot 1 quickly (single validator would be too easy, let's just fast-forward)
        manager.start_new_slot(1).await;
        manager.slot_timer = None;
        manager.run_spc(1);
        let (slot, v_high) = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
        manager.on_spc_v_high(slot, v_high).await;
        let _block1 = exec_rx.try_next().unwrap().unwrap();

        // Now at slot 2 — the pre-buffered proposal should still be there
        assert_eq!(manager.current_slot, 2);
        // Slot 2 state should have 2 proposals: own (from start_new_slot) + pre-buffered
        let slot2_state = manager.slot_states.get(&2).unwrap();
        assert_eq!(slot2_state.proposal_buffer().proposal_count(), 2);
    }
}
