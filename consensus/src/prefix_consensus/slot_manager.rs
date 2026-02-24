// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! SlotManager: main orchestrator for multi-slot prefix consensus (Algorithm 4).
//!
//! Runs one slot at a time: broadcasts proposals, collects them via [`SlotState`],
//! spawns SPC (stubbed in Phase 5), builds blocks from v_high, wraps in
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
    slot_types::{SlotConsensusMsg, SlotProposal, create_signed_slot_proposal},
};
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_signer::ValidatorSigner,
    validator_verifier::ValidatorVerifier,
};
use aptos_validator_transaction_pool as vtxn_pool;
use std::{collections::HashMap, pin::Pin, sync::Arc, time::Duration};
use tokio::{
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    time::Sleep,
};

/// Default 2Δ timeout for proposal collection.
const SLOT_PROPOSAL_TIMEOUT_MS: u64 = 300;

/// Output from an SPC instance for a given slot.
pub struct SPCOutput {
    pub slot: u64,
    pub v_high: PrefixVector,
}

/// Main orchestrator for multi-slot prefix consensus.
///
/// Runs one slot at a time: broadcasts proposals, collects them via
/// [`SlotState`], spawns SPC, builds blocks from v_high, wraps in
/// [`OrderedBlocks`], sends to execution, updates ranking, and advances.
pub struct SlotManager<NS: SubprotocolNetworkSender<SlotConsensusMsg>> {
    // Identity
    author: Author,
    epoch: u64,
    validator_signer: ValidatorSigner,
    validator_verifier: Arc<ValidatorVerifier>,

    // Slot state
    current_slot: u64,
    slot_states: HashMap<u64, SlotState>,
    ranking_manager: MultiSlotRankingManager,

    // Per-slot SPC channels
    spc_msg_tx: Option<UnboundedSender<(Author, StrongPrefixConsensusMsg)>>,
    spc_output_rx: Option<UnboundedReceiver<SPCOutput>>,

    // Execution bridge
    execution_channel: UnboundedSender<OrderedBlocks>,

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

impl<NS: SubprotocolNetworkSender<SlotConsensusMsg>> SlotManager<NS> {
    pub fn new(
        author: Author,
        epoch: u64,
        validator_signer: ValidatorSigner,
        validator_verifier: Arc<ValidatorVerifier>,
        ranking_manager: MultiSlotRankingManager,
        execution_channel: UnboundedSender<OrderedBlocks>,
        payload_client: Arc<dyn PayloadClient>,
        parent_block_info: BlockInfo,
        network_sender: NS,
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
        mut message_rx: UnboundedReceiver<(Author, SlotConsensusMsg)>,
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
                    if let Some(output) = output {
                        self.on_spc_v_high(output).await;
                    }
                }

                // Incoming messages
                Some((author, msg)) = message_rx.recv() => {
                    self.slot_timer = timer_opt;
                    self.spc_output_rx = spc_rx_opt;
                    match msg {
                        SlotConsensusMsg::SlotProposal(p) => {
                            self.process_proposal(author, *p).await;
                        }
                        SlotConsensusMsg::StrongPCMsg { slot, msg, .. } => {
                            self.process_spc_message(author, slot, msg);
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
        if let Err(e) = self
            .slot_states
            .get_mut(&slot)
            .expect("just inserted")
            .insert_proposal(proposal.clone())
        {
            warn!(epoch = self.epoch, slot = slot, error = ?e, "Failed to insert own proposal");
        }

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
        if let Err(e) = slot_state.insert_proposal(proposal) {
            debug!(
                epoch = self.epoch,
                slot = slot,
                error = ?e,
                "Failed to insert proposal"
            );
            return;
        }

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
    // (stub — Phase 6 replaces run_spc with real StrongPrefixConsensusManager)
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

        // Create channels
        let (spc_msg_tx, _spc_msg_rx) = mpsc::unbounded_channel();
        let (spc_output_tx, spc_output_rx) = mpsc::unbounded_channel();

        // STUB: spawn task that immediately returns input_vector as v_high.
        // Phase 6 replaces this with real StrongPrefixConsensusManager.
        let slot_copy = slot;
        tokio::spawn(async move {
            let _ = spc_output_tx.send(SPCOutput {
                slot: slot_copy,
                v_high: input_vector,
            });
        });

        self.spc_msg_tx = Some(spc_msg_tx);
        self.spc_output_rx = Some(spc_output_rx);
    }

    async fn on_spc_v_high(&mut self, spc_output: SPCOutput) {
        let SPCOutput { slot, v_high } = spc_output;
        info!(
            epoch = self.epoch,
            slot = slot,
            v_high_len = v_high.len(),
            "SPC completed, building block"
        );

        // Get payload_map from slot state.
        // TODO(Phase 7): v_high may contain hashes for proposals we never received
        // (other validators saw them and SPC committed them). Currently payload_map
        // only has payloads from locally received proposals. Phase 7 adds payload
        // resolution: check late-arriving proposals, then fetch missing payloads
        // from peers by hash. See .plans/phase7-payload-resolution.md.
        let payload_map = self
            .slot_states
            .get_mut(&slot)
            .and_then(|s| s.take_payload_map())
            .expect("payload_map missing — prepare_spc_input should have set it");

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
        if let Err(e) = self.execution_channel.send(ordered) {
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

        // Clean up slot state and SPC channels
        self.spc_msg_tx.take();
        self.spc_output_rx.take();
        self.slot_states.remove(&slot);

        // Advance to next slot
        self.start_new_slot(slot + 1).await;
    }

    fn process_spc_message(
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
        if let Some(tx) = &self.spc_msg_tx {
            let _ = tx.send((author, msg));
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
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    // ========================================================================
    // Test infrastructure
    // ========================================================================

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
        SlotManager<MockSlotNetworkSender>,
        UnboundedReceiver<OrderedBlocks>,
        MockSlotNetworkSender,
    ) {
        let (exec_tx, exec_rx) = mpsc::unbounded_channel();
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
        // SPC stub returns immediately → on_spc_v_high fires → block sent to execution
        // Give the stub task a moment to deliver
        tokio::task::yield_now().await;

        // Check if SPC output was processed (it may need one more yield)
        if manager.spc_output_rx.is_some() {
            // Process the SPC output manually
            let output = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
            manager.on_spc_v_high(output).await;
        }

        // Block should have been sent to execution
        let ordered = exec_rx.try_recv().expect("Block should be on execution channel");
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
        // SPC stub should fire immediately
        tokio::task::yield_now().await;

        if manager.spc_output_rx.is_some() {
            let output = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
            manager.on_spc_v_high(output).await;
        }

        // Block should be on exec channel
        let ordered = exec_rx.try_recv().expect("Block should be on execution channel");
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
        tokio::task::yield_now().await;
        let output = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();

        // v_high has length 4 (full ranking), entries for signers[2] and [3] are zero
        assert_eq!(output.v_high.len(), 4);
        manager.on_spc_v_high(output).await;

        // Consume the block
        let _block1 = exec_rx.try_recv().expect("Block 1");

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

        tokio::task::yield_now().await;
        if manager.spc_output_rx.is_some() {
            let output = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
            manager.on_spc_v_high(output).await;
        }

        let ordered = exec_rx.try_recv().expect("Block on exec channel");
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
        tokio::task::yield_now().await;
        if manager.spc_output_rx.is_some() {
            let output = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
            manager.on_spc_v_high(output).await;
        }
        let _block1 = exec_rx.try_recv().unwrap();

        // Parent should have been updated
        assert_ne!(manager.parent_block_info.id(), initial_parent.id());
        let parent_after_slot1 = manager.parent_block_info.clone();

        // Run slot 2
        tokio::task::yield_now().await;
        if manager.spc_output_rx.is_some() {
            let output = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
            manager.on_spc_v_high(output).await;
        }
        let _block2 = exec_rx.try_recv().unwrap();

        // Parent should have been updated again
        assert_ne!(manager.parent_block_info.id(), parent_after_slot1.id());
    }

    #[tokio::test]
    async fn test_close_signal_stops_manager() {
        let (signers, verifier) = create_validators(1);
        let (manager, _exec_rx, _ns) = build_test_manager(&signers, verifier);

        let (msg_tx, msg_rx) = mpsc::unbounded_channel();
        let (close_tx, close_rx) = oneshot::channel();
        let (ack_tx, ack_rx) = oneshot::channel();

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
        tokio::task::yield_now().await;
        let output = manager.spc_output_rx.as_mut().unwrap().recv().await.unwrap();
        manager.on_spc_v_high(output).await;
        let _block1 = exec_rx.try_recv().unwrap();

        // Now at slot 2 — the pre-buffered proposal should still be there
        assert_eq!(manager.current_slot, 2);
        // Slot 2 state should have 2 proposals: own (from start_new_slot) + pre-buffered
        let slot2_state = manager.slot_states.get(&2).unwrap();
        assert_eq!(slot2_state.proposal_buffer().proposal_count(), 2);
    }
}
