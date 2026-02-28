// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! ProxyConsensusHooks defines the interaction layer between the proxy RoundManager
//! and the primary consensus. When a second RoundManager instance runs proxy consensus
//! using standard Aptos BFT, these hooks handle proxy-specific behavior:
//!
//! 1. **Block data transformation**: Converting standard proposals to ProxyBlock variants
//!    with last_primary_proof_round and primary_proof fields.
//! 2. **Block ordering**: When proxy blocks are ordered by the BFT commit rule,
//!    detecting the cutting point and forwarding blocks to primary consensus.
//! 3. **Primary state updates**: Receiving QC/TC updates from primary consensus
//!    to track the current primary round.

use crate::{
    error::StateSyncError,
    network::IncomingCommitRequest,
    pipeline::{execution_client::TExecutionClient, pipeline_builder::PipelineBuilder},
};
use anyhow::anyhow;
use aptos_channels::aptos_channel;
use aptos_consensus_types::{
    block::Block,
    block_data::BlockData,
    common::{Author, Payload, Round},
    opt_block_data::OptBlockData,
    pipelined_block::PipelinedBlock,
    primary_consensus_proof::PrimaryConsensusProof,
    proxy_messages::OrderedProxyBlocksMsg,
    quorum_cert::QuorumCert,
    timeout_2chain::TwoChainTimeoutCertificate,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_executor_types::ExecutorResult;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_proxy_primary::{proxy_metrics, ProxyToPrimaryEvent};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    validator_txn::ValidatorTransaction,
};
use async_trait::async_trait;
use move_core_types::account_address::AccountAddress;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

/// Tracks the proxy's view of primary consensus state.
///
/// All fields are updated/consumed atomically under a single lock to ensure
/// exactly one proxy block consumes each primary proof (the cutting point).
struct ProxyPrimaryState {
    /// Pending primary proof (QC or TC) to be attached to the next proxy block.
    /// Consumed (taken) on first use — ensures only ONE block gets the proof,
    /// making it the cutting point (last block of the current primary round batch).
    pending_primary_proof: Option<Arc<PrimaryConsensusProof>>,
    /// Highest primary proof round we've ever received. Used for monotonicity enforcement
    /// — only accept proofs with strictly increasing rounds.
    highest_known_primary_proof_round: Round,
}

/// Hooks for proxy-specific behavior in the proxy RoundManager.
/// The primary RoundManager uses None (no hooks needed).
#[async_trait]
pub trait ProxyConsensusHooks: Send + Sync {
    /// Transform a generated proposal BlockData into a ProxyBlock variant.
    ///
    /// Called by the proxy RoundManager after ProposalGenerator creates a standard
    /// proposal. The hook:
    /// 1. Computes `last_primary_proof_round` from the parent block
    /// 2. Tries to attach the pending primary proof if available and valid
    /// 3. Returns a BlockData with BlockType::ProposalExt(ProposalExt::ProxyV0)
    ///
    /// `parent_last_primary_proof_round` is the `last_primary_proof_round` value
    /// of the parent block (the block certified by `quorum_cert`).
    fn transform_proposal(
        &self,
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        round: Round,
        timestamp: u64,
        quorum_cert: QuorumCert,
        parent_last_primary_proof_round: Round,
    ) -> BlockData;

    /// Transform an optimistic proposal into a proxy OptBlockData with ProxyV0 body.
    ///
    /// Called by the proxy RoundManager during optimistic proposal generation.
    /// The hook:
    /// 1. Computes `last_primary_proof_round` from the parent block
    /// 2. Tries to attach the pending primary proof if available and valid
    /// 3. Returns an OptBlockData with OptBlockBody::ProxyV0
    ///
    /// `parent_last_primary_proof_round` is the `last_primary_proof_round` value
    /// of the parent block.
    fn transform_opt_proposal(
        &self,
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        epoch: u64,
        round: Round,
        timestamp_usecs: u64,
        parent: aptos_types::block_info::BlockInfo,
        grandparent_qc: QuorumCert,
        parent_last_primary_proof_round: Round,
    ) -> OptBlockData;

    /// Called when proxy blocks are ordered (committed by the BFT commit rule).
    ///
    /// The hook detects the cutting point based on primary QC arrivals and
    /// constructs OrderedProxyBlocksMsg to forward to primary consensus.
    ///
    /// `committed_blocks` are the blocks committed in this round, in order.
    async fn on_ordered_blocks(&self, committed_blocks: Vec<Arc<Block>>);

    /// Update the latest primary QC received from primary consensus.
    /// This is used to determine the cutting point for block forwarding
    /// and to attach primary_qc to proxy proposals.
    fn update_primary_qc(&self, qc: Arc<QuorumCert>);

    /// Update the latest primary TC received from primary consensus.
    fn update_primary_tc(
        &self,
        tc: Arc<aptos_consensus_types::timeout_2chain::TwoChainTimeoutCertificate>,
    );
}

// =============================================================================
// ProxyHooksImpl - Concrete implementation of ProxyConsensusHooks
// =============================================================================

/// Concrete implementation of ProxyConsensusHooks for the proxy RoundManager.
///
/// This struct bridges the proxy RoundManager (running standard Aptos BFT) with
/// primary consensus by:
/// - Tracking primary QC/TC received from primary consensus
/// - Consuming primary QC exactly once to create cutting points
/// - Forwarding ordered proxy blocks to primary consensus via channel + network
pub struct ProxyHooksImpl {
    /// Combined primary state: pending proof, current primary round, monotonicity tracker.
    /// Protected by a single lock to ensure atomic consume-and-advance semantics.
    primary_state: Mutex<ProxyPrimaryState>,
    /// Buffer for ordered blocks from intermediate commits (no cutting point).
    /// The 3-chain commit rule may fire multiple times between cutting points,
    /// producing ordered batches without a cutting point. These blocks are buffered
    /// here and prepended to the next batch when a cutting point is committed.
    pending_ordered_blocks: Mutex<Vec<Block>>,
    /// Channel to send OrderedProxyBlocksMsg to primary RoundManager.
    ordered_blocks_tx: tokio::sync::mpsc::UnboundedSender<ProxyToPrimaryEvent>,
    /// Network sender for broadcasting ordered blocks to all primaries.
    /// None only in unit tests where we don't have a real network stack.
    network: Option<Arc<crate::network::NetworkSender>>,
    /// Shared flag indicating a primary proof is pending. Used by
    /// ProxyBudgetPayloadClient to skip backpressure delay for cutting-point blocks.
    has_pending_proof: Arc<AtomicBool>,
    /// This validator's identity. Used to gate network broadcast: only the
    /// proposer of the cutting-point block broadcasts to remote primaries.
    self_author: Author,
}

impl ProxyHooksImpl {
    pub fn new(
        ordered_blocks_tx: tokio::sync::mpsc::UnboundedSender<ProxyToPrimaryEvent>,
        network: Arc<crate::network::NetworkSender>,
        initial_primary_qc: Option<Arc<QuorumCert>>,
        has_pending_proof: Arc<AtomicBool>,
        self_author: Author,
    ) -> Self {
        let (pending_proof, highest_round) = match initial_primary_qc {
            Some(qc) => {
                let round = qc.certified_block().round();
                let proof = Arc::new(PrimaryConsensusProof::QC((*qc).clone()));
                has_pending_proof.store(true, Ordering::Release);
                (Some(proof), round)
            },
            None => (None, 0),
        };
        Self {
            primary_state: Mutex::new(ProxyPrimaryState {
                pending_primary_proof: pending_proof,
                highest_known_primary_proof_round: highest_round,
            }),
            pending_ordered_blocks: Mutex::new(Vec::new()),
            ordered_blocks_tx,
            network: Some(network),
            has_pending_proof,
            self_author,
        }
    }

    /// Atomically try to take the pending proof if it can advance the chain.
    ///
    /// `base_lppr` is the `last_primary_proof_round` inherited from the parent block.
    /// The proof is only consumed if `proof.round >= base_lppr + 1`.
    ///
    /// Returns `Some(proof)` if a valid proof was consumed, `None` otherwise.
    fn try_take_pending_proof(&self, base_lppr: Round) -> Option<PrimaryConsensusProof> {
        let mut state = self.primary_state.lock();
        if let Some(ref proof) = state.pending_primary_proof {
            if proof.proof_round() >= base_lppr + 1 {
                let proof = state.pending_primary_proof.take().unwrap();
                self.has_pending_proof.store(false, Ordering::Release);
                return Some((*proof).clone());
            }
            info!(
                proof_round = proof.proof_round(),
                base_lppr = base_lppr,
                "try_take_pending_proof: proof round not high enough to advance chain, keeping"
            );
        }
        None
    }

    /// Shared inner method for updating the pending primary proof (QC or TC).
    /// Enforces monotonicity: only accepts proofs with strictly increasing rounds.
    fn update_primary_proof_inner(&self, proof: PrimaryConsensusProof) {
        let mut state = self.primary_state.lock();
        let proof_round = proof.proof_round();
        if proof_round > state.highest_known_primary_proof_round {
            info!(
                proof_round = proof_round,
                prev_highest = state.highest_known_primary_proof_round,
                is_qc = proof.is_qc(),
                is_tc = proof.is_tc(),
                "ProxyHooksImpl: updating primary proof"
            );
            state.highest_known_primary_proof_round = proof_round;
            state.pending_primary_proof = Some(Arc::new(proof));
            self.has_pending_proof.store(true, Ordering::Release);
        } else {
            debug!(
                proof_round = proof_round,
                highest = state.highest_known_primary_proof_round,
                is_qc = proof.is_qc(),
                is_tc = proof.is_tc(),
                "ProxyHooksImpl: ignoring stale primary proof (not strictly increasing)"
            );
        }
    }
}

#[async_trait]
impl ProxyConsensusHooks for ProxyHooksImpl {
    fn transform_proposal(
        &self,
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        round: Round,
        timestamp: u64,
        quorum_cert: QuorumCert,
        parent_last_primary_proof_round: Round,
    ) -> BlockData {
        // Compute last_primary_proof_round from parent (base value)
        let base_lppr = parent_last_primary_proof_round;

        // Try to attach pending proof
        let primary_proof = self.try_take_pending_proof(base_lppr);
        let last_primary_proof_round = match &primary_proof {
            Some(proof) => proof.proof_round(),
            None => base_lppr,
        };

        // Proxy uses round-robin leader election, so failed_authors should always
        // be empty. If it's not, the proposer election is misconfigured.
        if !failed_authors.is_empty() {
            warn!(
                round = round,
                num_failed = failed_authors.len(),
                "ProxyHooksImpl: unexpected non-empty failed_authors in proxy block"
            );
        }

        info!(
            round = round,
            last_primary_proof_round = last_primary_proof_round,
            has_primary_proof = primary_proof.is_some(),
            "ProxyHooksImpl: transforming proposal to ProxyBlock"
        );

        proxy_metrics::PROXY_CONSENSUS_PROPOSALS_SENT.inc();

        BlockData::new_from_proxy(
            quorum_cert.certified_block().epoch(),
            round,
            timestamp,
            quorum_cert,
            validator_txns,
            payload,
            author,
            vec![], // Proxy blocks always have empty failed_authors (round-robin)
            last_primary_proof_round,
            primary_proof,
        )
    }

    fn transform_opt_proposal(
        &self,
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        epoch: u64,
        round: Round,
        timestamp_usecs: u64,
        parent: aptos_types::block_info::BlockInfo,
        grandparent_qc: QuorumCert,
        parent_last_primary_proof_round: Round,
    ) -> OptBlockData {
        // Compute last_primary_proof_round from parent (base value)
        let base_lppr = parent_last_primary_proof_round;

        // Try to attach pending proof
        let primary_proof = self.try_take_pending_proof(base_lppr);
        let last_primary_proof_round = match &primary_proof {
            Some(proof) => proof.proof_round(),
            None => base_lppr,
        };

        info!(
            round = round,
            last_primary_proof_round = last_primary_proof_round,
            has_primary_proof = primary_proof.is_some(),
            "ProxyHooksImpl: transforming opt proposal to ProxyV0"
        );

        proxy_metrics::PROXY_CONSENSUS_PROPOSALS_SENT.inc();

        OptBlockData::new_proxy(
            validator_txns,
            payload,
            author,
            epoch,
            round,
            timestamp_usecs,
            parent,
            grandparent_qc,
            last_primary_proof_round,
            primary_proof,
        )
    }

    async fn on_ordered_blocks(&self, committed_blocks: Vec<Arc<Block>>) {
        if committed_blocks.is_empty() {
            return;
        }

        // Collect blocks sorted by round (ascending = oldest-first)
        let mut blocks: Vec<Block> = committed_blocks
            .iter()
            .map(|b| (**b).clone())
            .collect();
        blocks.sort_by_key(|b| b.round());

        // Assert strictly increasing rounds (committed blocks should never have duplicates)
        for i in 1..blocks.len() {
            assert!(
                blocks[i].round() > blocks[i - 1].round(),
                "on_ordered_blocks: blocks not in strictly ascending round order: round {} <= {}",
                blocks[i].round(),
                blocks[i - 1].round(),
            );
        }

        // Find the last cutting point. There may be multiple cutting points in a batch
        // (the execution pipeline can batch multiple committed rounds), but we only need
        // the last one — all blocks up to it go in a single OrderedProxyBlocksMsg.
        // The validation allows non-decreasing last_primary_proof_round values
        // across blocks within the batch.
        let cut_idx = blocks
            .iter()
            .rposition(|b| b.block_data().primary_proof().is_some());

        if cut_idx.is_none() {
            // No cutting point in this batch — buffer blocks for the next batch.
            // The 3-chain commit rule may fire multiple times between cutting points.
            let num = blocks.len();
            self.pending_ordered_blocks.lock().append(&mut blocks);
            info!(
                "ProxyHooksImpl: buffered {} ordered blocks (no cutting point), \
                 total buffered: {}",
                num,
                self.pending_ordered_blocks.lock().len(),
            );
            return;
        }
        let cut_idx = cut_idx.unwrap();

        // Split: blocks up to and including cutting point go out now;
        // blocks after the cutting point go into the buffer for next batch.
        let after_cut = blocks.split_off(cut_idx + 1);

        // Prepend buffered blocks from previous intermediate commits
        let mut buffered = std::mem::take(&mut *self.pending_ordered_blocks.lock());
        buffered.append(&mut blocks);
        let blocks = buffered;

        // Buffer blocks after cutting point for the next batch
        if !after_cut.is_empty() {
            info!(
                "ProxyHooksImpl: buffering {} blocks after cutting point for next batch",
                after_cut.len(),
            );
            *self.pending_ordered_blocks.lock() = after_cut;
        }

        // Extract primary_proof and author from the cutting-point block
        let cutting_block = blocks.last().expect("blocks is non-empty after prepend");
        let primary_proof = cutting_block
            .block_data()
            .primary_proof()
            .cloned()
            .expect("cutting-point block must have primary_proof");
        let cutting_block_author = cutting_block.author();
        let primary_round = primary_proof.proof_round() + 1;

        proxy_metrics::PROXY_CONSENSUS_BLOCKS_ORDERED.inc_by(blocks.len() as u64);

        info!(
            num_blocks = blocks.len(),
            primary_round = primary_round,
            "ProxyHooksImpl: forwarding ordered proxy blocks"
        );

        // Construct the ordered proxy blocks message
        let ordered_msg = OrderedProxyBlocksMsg::new(
            blocks,
            primary_proof,
        );

        // Only the proposer of the cutting-point block broadcasts to remote primaries.
        // All validators still send via local channel so their own primary gets blocks
        // instantly without waiting for a network round-trip.
        if cutting_block_author == Some(self.self_author) {
            if let Some(network) = &self.network {
                network
                    .broadcast_ordered_proxy_blocks(ordered_msg.clone())
                    .await;
            }
        }

        // Always send via local channel to own primary RoundManager
        let _ = self
            .ordered_blocks_tx
            .send(ProxyToPrimaryEvent::OrderedProxyBlocks(ordered_msg));

        proxy_metrics::PROXY_CONSENSUS_BLOCKS_FORWARDED.inc();
    }

    fn update_primary_qc(&self, qc: Arc<QuorumCert>) {
        self.update_primary_proof_inner(PrimaryConsensusProof::QC((*qc).clone()));
    }

    fn update_primary_tc(&self, tc: Arc<TwoChainTimeoutCertificate>) {
        self.update_primary_proof_inner(PrimaryConsensusProof::TC((*tc).clone()));
    }
}

// =============================================================================
// ProxyExecutionClient - TExecutionClient implementation for proxy RoundManager
// =============================================================================

/// A lightweight execution client for the proxy RoundManager that forwards
/// ordered blocks through ProxyConsensusHooks instead of executing them.
///
/// This replaces DummyExecutionClient for the proxy BlockStore. When blocks
/// are finalized (ordered by BFT commit rule), it converts them to plain
/// Block types and delegates to the proxy hooks for forwarding to primary.
pub struct ProxyExecutionClient {
    hooks: Arc<dyn ProxyConsensusHooks>,
}

impl ProxyExecutionClient {
    pub fn new(hooks: Arc<dyn ProxyConsensusHooks>) -> Self {
        Self { hooks }
    }
}

#[async_trait]
impl TExecutionClient for ProxyExecutionClient {
    async fn start_epoch(
        &self,
        _maybe_consensus_key: Arc<aptos_crypto::bls12381::PrivateKey>,
        _epoch_state: Arc<aptos_types::epoch_state::EpochState>,
        _commit_signer_provider: Arc<dyn crate::pipeline::signing_phase::CommitSignerProvider>,
        _payload_manager: Arc<dyn crate::payload_manager::TPayloadManager>,
        _onchain_consensus_config: &aptos_types::on_chain_config::OnChainConsensusConfig,
        _onchain_execution_config: &aptos_types::on_chain_config::OnChainExecutionConfig,
        _onchain_randomness_config: &aptos_types::on_chain_config::OnChainRandomnessConfig,
        _rand_config: Option<crate::rand::rand_gen::types::RandConfig>,
        _fast_rand_config: Option<crate::rand::rand_gen::types::RandConfig>,
        _rand_msg_rx: aptos_channel::Receiver<AccountAddress, crate::network::IncomingRandGenRequest>,
        _secret_sharing_msg_rx: aptos_channel::Receiver<AccountAddress, crate::network::IncomingSecretShareRequest>,
        _highest_committed_round: Round,
    ) {
    }

    fn get_execution_channel(
        &self,
    ) -> Option<futures::channel::mpsc::UnboundedSender<crate::pipeline::buffer_manager::OrderedBlocks>>
    {
        None
    }

    async fn finalize_order(
        &self,
        blocks: Vec<Arc<PipelinedBlock>>,
        _ordered_proof: WrappedLedgerInfo,
    ) -> ExecutorResult<()> {
        let blocks: Vec<Arc<Block>> = blocks
            .iter()
            .map(|b| Arc::new(b.block().clone()))
            .collect();
        self.hooks.on_ordered_blocks(blocks).await;
        Ok(())
    }

    fn send_commit_msg(
        &self,
        _peer_id: AccountAddress,
        _commit_msg: IncomingCommitRequest,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn sync_for_duration(
        &self,
        _duration: Duration,
    ) -> Result<LedgerInfoWithSignatures, StateSyncError> {
        Err(StateSyncError::from(anyhow!(
            "sync_for_duration() is not supported by the ProxyExecutionClient!"
        )))
    }

    async fn sync_to_target(
        &self,
        _target: LedgerInfoWithSignatures,
    ) -> Result<(), StateSyncError> {
        Ok(())
    }

    async fn reset(&self, _target: &LedgerInfoWithSignatures) -> anyhow::Result<()> {
        Ok(())
    }

    async fn end_epoch(&self) {}

    fn pipeline_builder(
        &self,
        _signer: Arc<aptos_types::validator_signer::ValidatorSigner>,
    ) -> PipelineBuilder {
        unreachable!("ProxyExecutionClient does not support pipeline_builder")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_consensus_types::{
        block::Block,
        block_data::BlockData,
        common::Payload,
        vote_data::VoteData,
    };
    use aptos_crypto::HashValue;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_info::BlockInfo,
        validator_signer::ValidatorSigner,
    };

    fn make_qc(epoch: u64, round: Round) -> QuorumCert {
        let block_info =
            BlockInfo::new(epoch, round, HashValue::random(), HashValue::random(), 0, 0, None);
        let vote_data = VoteData::new(block_info.clone(), block_info.clone());
        let ledger_info =
            aptos_types::ledger_info::LedgerInfo::new(block_info, HashValue::zero());
        let li_sig = aptos_types::ledger_info::LedgerInfoWithSignatures::new(
            ledger_info,
            AggregateSignature::empty(),
        );
        QuorumCert::new(vote_data, li_sig)
    }

    fn make_qc_for_block(epoch: u64, round: Round, block_id: HashValue) -> QuorumCert {
        let block_info =
            BlockInfo::new(epoch, round, block_id, HashValue::random(), 0, 0, None);
        let parent_info = BlockInfo::new(
            epoch,
            round.saturating_sub(1),
            HashValue::random(),
            HashValue::random(),
            0,
            0,
            None,
        );
        let vote_data = VoteData::new(block_info.clone(), parent_info);
        let ledger_info =
            aptos_types::ledger_info::LedgerInfo::new(block_info, HashValue::zero());
        let li_sig = aptos_types::ledger_info::LedgerInfoWithSignatures::new(
            ledger_info,
            AggregateSignature::empty(),
        );
        QuorumCert::new(vote_data, li_sig)
    }

    fn make_proxy_block(
        signer: &ValidatorSigner,
        round: Round,
        parent_qc: QuorumCert,
        last_primary_proof_round: Round,
        primary_proof: Option<PrimaryConsensusProof>,
    ) -> Block {
        let block_data = BlockData::new_from_proxy(
            1, round,
            aptos_infallible::duration_since_epoch().as_micros() as u64,
            parent_qc,
            vec![],
            Payload::empty(true, true),
            signer.author(),
            vec![],
            last_primary_proof_round,
            primary_proof,
        );
        Block::new_proposal_from_block_data(block_data, signer).unwrap()
    }

    fn make_proxy_block_chain(
        signer: &ValidatorSigner,
        num_blocks: usize,
        start_round: Round,
        last_primary_proof_round: Round,
        primary_qc: Option<QuorumCert>,
    ) -> Vec<Block> {
        assert!(num_blocks > 0);
        let mut blocks = Vec::with_capacity(num_blocks);

        let genesis_qc = make_qc(1, 0);
        let is_last = num_blocks == 1;
        let first_proof = if is_last { primary_qc.as_ref().map(|qc| PrimaryConsensusProof::QC(qc.clone())) } else { None };
        let lppr = if is_last { first_proof.as_ref().map_or(last_primary_proof_round, |p| p.proof_round()) } else { last_primary_proof_round };
        let first = make_proxy_block(signer, start_round, genesis_qc, lppr, first_proof);
        blocks.push(first);

        for i in 1..num_blocks {
            let prev = &blocks[i - 1];
            let parent_qc = make_qc_for_block(1, prev.round(), prev.id());
            let is_last = i == num_blocks - 1;
            let proof = if is_last { primary_qc.as_ref().map(|qc| PrimaryConsensusProof::QC(qc.clone())) } else { None };
            let lppr = if is_last { proof.as_ref().map_or(last_primary_proof_round, |p| p.proof_round()) } else { last_primary_proof_round };
            let block = make_proxy_block(
                signer,
                start_round + i as u64,
                parent_qc,
                lppr,
                proof,
            );
            blocks.push(block);
        }

        blocks
    }

    /// Create a ProxyHooksImpl with test channels.
    /// Network is set to None since we only test non-network methods here.
    /// Network broadcasting is tested via the Forge E2E test.
    fn create_test_hooks(
        initial_primary_qc: Option<Arc<QuorumCert>>,
    ) -> (
        ProxyHooksImpl,
        tokio::sync::mpsc::UnboundedReceiver<aptos_proxy_primary::ProxyToPrimaryEvent>,
    ) {
        let (ordered_blocks_tx, ordered_blocks_rx) = tokio::sync::mpsc::unbounded_channel();

        let (pending_proof, highest_round) = match initial_primary_qc {
            Some(qc) => {
                let round = qc.certified_block().round();
                let proof = Arc::new(PrimaryConsensusProof::QC((*qc).clone()));
                (Some(proof), round)
            },
            None => (None, 0),
        };

        let has_pending_proof = Arc::new(AtomicBool::new(pending_proof.is_some()));
        (
            ProxyHooksImpl {
                primary_state: aptos_infallible::Mutex::new(ProxyPrimaryState {
                    pending_primary_proof: pending_proof,
                    highest_known_primary_proof_round: highest_round,
                }),
                pending_ordered_blocks: aptos_infallible::Mutex::new(Vec::new()),
                ordered_blocks_tx,
                network: None,
                has_pending_proof,
                self_author: AccountAddress::ZERO,
            },
            ordered_blocks_rx,
        )
    }

    // =========================================================================
    // try_take_pending_proof tests
    // =========================================================================

    #[test]
    fn test_try_take_pending_proof_with_pending_qc() {
        // Initial QC at round 5 → state: pending=QC(5), highest=5
        let primary_qc = Arc::new(make_qc(1, 5));
        let (hooks, _rx) = create_test_hooks(Some(primary_qc));

        // Parent has lppr=0, so proof.round(5) >= 0 + 1 → should take
        let proof = hooks.try_take_pending_proof(0);
        assert!(proof.is_some(), "Should consume pending QC");
        assert_eq!(proof.unwrap().proof_round(), 5);
    }

    #[test]
    fn test_try_take_pending_proof_consumes_once() {
        // Initial QC at round 5
        let primary_qc = Arc::new(make_qc(1, 5));
        let (hooks, _rx) = create_test_hooks(Some(primary_qc));

        // First call consumes the QC
        let proof1 = hooks.try_take_pending_proof(0);
        assert!(proof1.is_some(), "First call should return QC");

        // Second call: QC was consumed
        let proof2 = hooks.try_take_pending_proof(5);
        assert!(proof2.is_none(), "Second call should NOT return QC (consumed)");
    }

    #[test]
    fn test_try_take_pending_proof_rejects_low_round() {
        // Initial QC at round 5
        let primary_qc = Arc::new(make_qc(1, 5));
        let (hooks, _rx) = create_test_hooks(Some(primary_qc));

        // Parent has lppr=5, so proof.round(5) < 5 + 1 = 6 → should NOT take
        let proof = hooks.try_take_pending_proof(5);
        assert!(proof.is_none(), "Should NOT consume proof when round not high enough");

        // Parent has lppr=4, so proof.round(5) >= 4 + 1 → should take
        let proof = hooks.try_take_pending_proof(4);
        assert!(proof.is_some());
    }

    #[test]
    fn test_try_take_pending_proof_consecutive_rounds() {
        // Start with QC(0)
        let initial_qc = Arc::new(make_qc(1, 0));
        let (hooks, _rx) = create_test_hooks(Some(initial_qc));

        // Consume QC(0) with parent lppr=0: proof.round(0) >= 0+1 is false → no take
        let proof = hooks.try_take_pending_proof(0);
        assert!(proof.is_none(), "QC(0) can't advance from lppr=0 (needs >= 1)");

        // Note: QC(0) is the genesis proof. Normally the parent would have lppr=0
        // and we'd need proof.round >= 1 to advance. So QC(0) can only be consumed
        // when the parent's lppr was before genesis (which doesn't happen).
        // In practice, the initial QC is consumed during initialization.
    }

    // =========================================================================
    // transform_proposal tests
    // =========================================================================

    #[test]
    fn test_transform_proposal_creates_proxy_block() {
        let primary_qc = Arc::new(make_qc(1, 5));
        let (hooks, _rx) = create_test_hooks(Some(primary_qc));

        let signer = ValidatorSigner::from_int(0);
        let parent_qc = make_qc(1, 10);

        let block_data = hooks.transform_proposal(
            vec![],
            Payload::empty(true, true),
            signer.author(),
            vec![],
            11, // round
            1000,
            parent_qc,
            0, // parent_last_primary_proof_round
        );

        assert!(block_data.is_proxy_block(), "Should create a ProxyBlock");
        assert_eq!(block_data.round(), 11);
    }

    #[test]
    fn test_transform_proposal_attaches_and_consumes_qc() {
        // Primary QC at round 5 → state: pending=QC(5), highest=5
        let primary_qc = Arc::new(make_qc(1, 5));
        let (hooks, _rx) = create_test_hooks(Some(primary_qc));

        let signer = ValidatorSigner::from_int(0);

        // First proposal: parent has lppr=0, proof.round(5) >= 1 → attach
        let block_data1 = hooks.transform_proposal(
            vec![],
            Payload::empty(true, true),
            signer.author(),
            vec![],
            11,
            1000,
            make_qc(1, 10),
            0, // parent_last_primary_proof_round
        );

        assert_eq!(block_data1.last_primary_proof_round(), Some(5));
        assert!(
            block_data1.primary_proof().is_some(),
            "First proposal should attach pending proof"
        );
        assert_eq!(block_data1.primary_proof().unwrap().proof_round(), 5);

        // Second proposal: proof was consumed, parent now has lppr=5
        let block_data2 = hooks.transform_proposal(
            vec![],
            Payload::empty(true, true),
            signer.author(),
            vec![],
            12,
            2000,
            make_qc(1, 11),
            5, // parent_last_primary_proof_round (from first block)
        );

        assert_eq!(block_data2.last_primary_proof_round(), Some(5), "lppr should inherit from parent");
        assert!(
            block_data2.primary_proof().is_none(),
            "Second proposal should NOT have proof (consumed by first)"
        );
    }

    #[test]
    fn test_transform_proposal_no_qc_when_none_available() {
        let (hooks, _rx) = create_test_hooks(None); // No primary QC

        let signer = ValidatorSigner::from_int(0);
        let parent_qc = make_qc(1, 10);

        let block_data = hooks.transform_proposal(
            vec![],
            Payload::empty(true, true),
            signer.author(),
            vec![],
            11,
            1000,
            parent_qc,
            0, // parent_last_primary_proof_round
        );

        assert_eq!(block_data.last_primary_proof_round(), Some(0), "lppr should be 0 (inherited from parent)");
        assert!(
            block_data.primary_proof().is_none(),
            "Should not attach primary proof when none available"
        );
    }

    // =========================================================================
    // Proposer lppr derivation safety rule tests
    // =========================================================================

    #[test]
    fn test_lppr_equals_proof_round_when_attached() {
        // When proof is attached, the block's lppr must equal proof.round
        let primary_qc = Arc::new(make_qc(1, 7));
        let (hooks, _rx) = create_test_hooks(Some(primary_qc));

        let signer = ValidatorSigner::from_int(0);
        let block_data = hooks.transform_proposal(
            vec![], Payload::empty(true, true), signer.author(), vec![],
            11, 1000, make_qc(1, 10),
            0, // parent_last_primary_proof_round
        );

        assert!(block_data.primary_proof().is_some(), "Proof should be attached");
        assert_eq!(
            block_data.last_primary_proof_round(), Some(7),
            "lppr must equal proof.round (7) when proof is attached"
        );
        assert_eq!(block_data.primary_proof().unwrap().proof_round(), 7);
    }

    #[test]
    fn test_lppr_equals_parent_lppr_when_no_proof() {
        // When no proof attached, lppr must equal parent's lppr
        let (hooks, _rx) = create_test_hooks(None);

        let signer = ValidatorSigner::from_int(0);
        let parent_lppr = 5;
        let block_data = hooks.transform_proposal(
            vec![], Payload::empty(true, true), signer.author(), vec![],
            11, 1000, make_qc(1, 10),
            parent_lppr,
        );

        assert!(block_data.primary_proof().is_none());
        assert_eq!(
            block_data.last_primary_proof_round(), Some(parent_lppr),
            "lppr must equal parent's lppr ({}) when no proof attached",
            parent_lppr,
        );
    }

    #[test]
    fn test_proof_not_attached_when_round_too_low() {
        // Proof.round (3) < parent_lppr (3) + 1 = 4 → proof NOT attached
        let primary_qc = Arc::new(make_qc(1, 3));
        let (hooks, _rx) = create_test_hooks(Some(primary_qc));

        let signer = ValidatorSigner::from_int(0);
        let block_data = hooks.transform_proposal(
            vec![], Payload::empty(true, true), signer.author(), vec![],
            11, 1000, make_qc(1, 10),
            3, // parent_lppr = 3, need proof.round >= 4
        );

        assert!(block_data.primary_proof().is_none(), "Proof should NOT be attached when round too low");
        assert_eq!(block_data.last_primary_proof_round(), Some(3), "lppr should be parent's lppr");
    }

    #[test]
    fn test_proof_attached_at_exact_threshold() {
        // Proof.round (4) == parent_lppr (3) + 1 → proof attached
        let primary_qc = Arc::new(make_qc(1, 4));
        let (hooks, _rx) = create_test_hooks(Some(primary_qc));

        let signer = ValidatorSigner::from_int(0);
        let block_data = hooks.transform_proposal(
            vec![], Payload::empty(true, true), signer.author(), vec![],
            11, 1000, make_qc(1, 10),
            3, // parent_lppr = 3, proof.round=4 >= 3+1 → attach
        );

        assert!(block_data.primary_proof().is_some(), "Proof should be attached at exact threshold");
        assert_eq!(block_data.last_primary_proof_round(), Some(4));
    }

    // =========================================================================
    // update_primary_qc / update_primary_tc tests
    // =========================================================================

    #[test]
    fn test_update_primary_qc_stores_pending() {
        let (hooks, _rx) = create_test_hooks(None);

        hooks.update_primary_qc(Arc::new(make_qc(1, 10)));

        // After update, try_take_pending_proof should return the new QC
        let proof = hooks.try_take_pending_proof(0);
        assert!(proof.is_some(), "Should have pending QC after update");
        assert_eq!(proof.unwrap().proof_round(), 10);
    }

    #[test]
    fn test_update_primary_qc_monotonicity() {
        let (hooks, _rx) = create_test_hooks(None);

        // Accept QC(10)
        hooks.update_primary_qc(Arc::new(make_qc(1, 10)));

        // Reject QC(5) — stale, lower round
        hooks.update_primary_qc(Arc::new(make_qc(1, 5)));

        // Reject QC(10) — not strictly increasing (equal)
        hooks.update_primary_qc(Arc::new(make_qc(1, 10)));

        // Verify we still have QC(10) not QC(5)
        let proof = hooks.try_take_pending_proof(0);
        assert!(proof.is_some());
        assert_eq!(proof.unwrap().proof_round(), 10);

        // Accept QC(15) — strictly increasing
        hooks.update_primary_qc(Arc::new(make_qc(1, 15)));
        let proof = hooks.try_take_pending_proof(10);
        assert!(proof.is_some());
        assert_eq!(proof.unwrap().proof_round(), 15);
    }

    #[test]
    fn test_update_primary_tc() {
        let (hooks, _rx) = create_test_hooks(None);

        // Just verify it doesn't panic
        let timeout = aptos_consensus_types::timeout_2chain::TwoChainTimeout::new(
            1, // epoch
            1, // round
            QuorumCert::dummy(),
        );
        let tc = Arc::new(
            aptos_consensus_types::timeout_2chain::TwoChainTimeoutCertificate::new(timeout),
        );
        hooks.update_primary_tc(tc);
    }

    // =========================================================================
    // on_ordered_blocks cutting point detection tests
    // =========================================================================
    // Note: Full on_ordered_blocks tests require a NetworkSender which is hard
    // to construct in unit tests. We test the cutting point logic indirectly
    // through the E2E forge test. Here we test the block analysis logic by
    // verifying the blocks that would be selected.

    #[test]
    fn test_cutting_point_detection_last_block_with_qc() {
        let signer = ValidatorSigner::from_int(0);
        let primary_qc = make_qc(1, 1);

        // 3 blocks: only last has primary_qc
        let blocks = make_proxy_block_chain(&signer, 3, 1, 2, Some(primary_qc));

        // Verify the last block has primary_proof
        assert!(blocks[0].block_data().primary_proof().is_none());
        assert!(blocks[1].block_data().primary_proof().is_none());
        assert!(blocks[2].block_data().primary_proof().is_some());

        // Simulate the cutting point detection from on_ordered_blocks
        let mut sorted: Vec<Block> = blocks.iter().map(|b| (*b).clone()).collect();
        sorted.sort_by_key(|b| b.round());

        let has_cutting_point = sorted.iter().any(|b| b.block_data().primary_proof().is_some());
        assert!(has_cutting_point);

        let cut_idx = sorted
            .iter()
            .rposition(|b| b.block_data().primary_proof().is_some())
            .unwrap();
        assert_eq!(cut_idx, 2); // last block
        sorted.truncate(cut_idx + 1);
        assert_eq!(sorted.len(), 3); // All 3 blocks included
    }

    #[test]
    fn test_cutting_point_detection_middle_block_with_proof() {
        let signer = ValidatorSigner::from_int(0);
        let primary_qc = make_qc(1, 1);

        // Create 3 blocks where the middle one has primary_proof
        let block1 = make_proxy_block(&signer, 1, make_qc(1, 0), 0, None);
        let block2 = make_proxy_block(
            &signer,
            2,
            make_qc_for_block(1, 1, block1.id()),
            1, // lppr = proof round
            Some(PrimaryConsensusProof::QC(primary_qc)),
        );
        let block3 = make_proxy_block(
            &signer,
            3,
            make_qc_for_block(1, 2, block2.id()),
            1, // inherited lppr
            None,
        );

        let mut sorted = vec![block1, block2, block3];
        sorted.sort_by_key(|b| b.round());

        let cut_idx = sorted
            .iter()
            .rposition(|b| b.block_data().primary_proof().is_some())
            .unwrap();
        assert_eq!(cut_idx, 1); // middle block (index 1)
        sorted.truncate(cut_idx + 1);
        assert_eq!(sorted.len(), 2); // Only blocks 1 and 2 included
    }

    #[test]
    fn test_no_cutting_point_when_no_proof() {
        let signer = ValidatorSigner::from_int(0);

        // 3 blocks, none with primary_proof
        let block1 = make_proxy_block(&signer, 1, make_qc(1, 0), 0, None);
        let block2 = make_proxy_block(
            &signer,
            2,
            make_qc_for_block(1, 1, block1.id()),
            0,
            None,
        );
        let block3 = make_proxy_block(
            &signer,
            3,
            make_qc_for_block(1, 2, block2.id()),
            0,
            None,
        );

        let sorted = vec![block1, block2, block3];
        let has_cutting_point = sorted.iter().any(|b| b.block_data().primary_proof().is_some());
        assert!(!has_cutting_point, "No block has primary_proof → no cutting point");
    }

    #[test]
    fn test_multiple_proof_blocks_uses_last() {
        let signer = ValidatorSigner::from_int(0);
        let primary_qc1 = make_qc(1, 1);
        let primary_qc2 = make_qc(1, 1);

        // Two blocks with primary_proof
        let block1 = make_proxy_block(&signer, 1, make_qc(1, 0), 1, Some(PrimaryConsensusProof::QC(primary_qc1)));
        let block2 = make_proxy_block(
            &signer,
            2,
            make_qc_for_block(1, 1, block1.id()),
            1, // lppr = proof round
            Some(PrimaryConsensusProof::QC(primary_qc2)),
        );
        let block3 = make_proxy_block(
            &signer,
            3,
            make_qc_for_block(1, 2, block2.id()),
            1, // inherited lppr
            None,
        );

        let mut sorted = vec![block1, block2, block3];
        sorted.sort_by_key(|b| b.round());

        // rposition should find block2 (index 1), not block1 (index 0)
        let cut_idx = sorted
            .iter()
            .rposition(|b| b.block_data().primary_proof().is_some())
            .unwrap();
        assert_eq!(cut_idx, 1);
        sorted.truncate(cut_idx + 1);
        assert_eq!(sorted.len(), 2); // blocks 1 and 2 included, block 3 excluded
    }

    #[test]
    fn test_blocks_sorted_by_round() {
        let signer = ValidatorSigner::from_int(0);
        let primary_qc = make_qc(1, 1);

        // Create blocks in reverse order
        let block3 = make_proxy_block(&signer, 3, make_qc(1, 2), 1, Some(PrimaryConsensusProof::QC(primary_qc)));
        let block1 = make_proxy_block(&signer, 1, make_qc(1, 0), 0, None);

        let mut blocks = vec![block3.clone(), block1.clone()];
        blocks.sort_by_key(|b| b.round());

        assert_eq!(blocks[0].round(), 1);
        assert_eq!(blocks[1].round(), 3);
    }

    // =========================================================================
    // on_ordered_blocks buffering tests
    // =========================================================================

    #[tokio::test]
    async fn test_on_ordered_blocks_buffers_intermediate_commits() {
        let signer = ValidatorSigner::from_int(0);
        let (hooks, mut rx) = create_test_hooks(None);

        // Intermediate commit: blocks 1, 2 with no cutting point → should be buffered
        let block1 = Arc::new(make_proxy_block(&signer, 1, make_qc(1, 0), 0, None));
        let block2 = Arc::new(make_proxy_block(
            &signer, 2, make_qc_for_block(1, 1, block1.id()), 0, None,
        ));
        hooks.on_ordered_blocks(vec![block1.clone(), block2.clone()]).await;

        // Nothing sent to primary yet
        assert!(rx.try_recv().is_err(), "No message should be sent without cutting point");

        // Verify buffered
        assert_eq!(hooks.pending_ordered_blocks.lock().len(), 2);

        // Next commit: block 3 with cutting point → should flush buffer + block 3
        let primary_qc = make_qc(1, 1);
        let block3 = Arc::new(make_proxy_block(
            &signer, 3, make_qc_for_block(1, 2, block2.id()), 1, Some(PrimaryConsensusProof::QC(primary_qc)),
        ));
        hooks.on_ordered_blocks(vec![block3.clone()]).await;

        // Buffer should be empty now
        assert_eq!(hooks.pending_ordered_blocks.lock().len(), 0);

        // Should have received the message with all 3 blocks
        let event = rx.try_recv().expect("Should receive ordered blocks message");
        match event {
            ProxyToPrimaryEvent::OrderedProxyBlocks(msg) => {
                assert_eq!(msg.proxy_blocks().len(), 3, "Should include buffered + cutting point");
                assert_eq!(msg.proxy_blocks()[0].round(), 1);
                assert_eq!(msg.proxy_blocks()[1].round(), 2);
                assert_eq!(msg.proxy_blocks()[2].round(), 3);
            },
        }
    }

    #[tokio::test]
    async fn test_on_ordered_blocks_buffers_after_cutting_point() {
        let signer = ValidatorSigner::from_int(0);
        let (hooks, mut rx) = create_test_hooks(None);

        // Batch with cutting point at block 2, block 3 after it
        let block1 = Arc::new(make_proxy_block(&signer, 1, make_qc(1, 0), 0, None));
        let primary_qc = make_qc(1, 1);
        let block2 = Arc::new(make_proxy_block(
            &signer, 2, make_qc_for_block(1, 1, block1.id()), 1, Some(PrimaryConsensusProof::QC(primary_qc)),
        ));
        let block3 = Arc::new(make_proxy_block(
            &signer, 3, make_qc_for_block(1, 2, block2.id()), 1, None,
        ));
        hooks.on_ordered_blocks(vec![block1.clone(), block2.clone(), block3.clone()]).await;

        // Message should contain blocks 1 and 2 (up to cutting point)
        let event = rx.try_recv().expect("Should receive ordered blocks message");
        match event {
            ProxyToPrimaryEvent::OrderedProxyBlocks(msg) => {
                assert_eq!(msg.proxy_blocks().len(), 2);
                assert_eq!(msg.proxy_blocks()[0].round(), 1);
                assert_eq!(msg.proxy_blocks()[1].round(), 2);
            },
        }

        // Block 3 should be buffered for next batch
        assert_eq!(hooks.pending_ordered_blocks.lock().len(), 1);
        assert_eq!(hooks.pending_ordered_blocks.lock()[0].round(), 3);
    }
}
