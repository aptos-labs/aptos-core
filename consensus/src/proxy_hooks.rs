// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! ProxyConsensusHooks defines the interaction layer between the proxy RoundManager
//! and the primary consensus. When a second RoundManager instance runs proxy consensus
//! using standard Aptos BFT, these hooks handle proxy-specific behavior:
//!
//! 1. **Block data transformation**: Converting standard proposals to ProxyBlock variants.
//! 2. **Block ordering**: When proxy blocks are ordered by the BFT commit rule,
//!    unconditionally forwarding ALL ordered blocks to primary consensus.
//!    The primary leader decides where to cut at proposal time (leader-driven cutting).

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
    proxy_messages::OrderedProxyBlocksMsg,
    quorum_cert::QuorumCert,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use aptos_executor_types::ExecutorResult;
use aptos_logger::prelude::*;
use aptos_proxy_primary::{proxy_metrics, ProxyToPrimaryEvent};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures,
    validator_txn::ValidatorTransaction,
};
use async_trait::async_trait;
use move_core_types::account_address::AccountAddress;
use std::sync::Arc;
use std::time::Duration;

/// Hooks for proxy-specific behavior in the proxy RoundManager.
/// The primary RoundManager uses None (no hooks needed).
#[async_trait]
pub trait ProxyConsensusHooks: Send + Sync {
    /// Transform a generated proposal BlockData into a ProxyBlock variant.
    ///
    /// Called by the proxy RoundManager after ProposalGenerator creates a standard
    /// proposal. The hook returns a BlockData with BlockType::ProposalExt(ProposalExt::ProxyV0).
    fn transform_proposal(
        &self,
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        round: Round,
        timestamp: u64,
        quorum_cert: QuorumCert,
    ) -> BlockData;

    /// Transform an optimistic proposal into a proxy OptBlockData with ProxyV0 body.
    ///
    /// Called by the proxy RoundManager during optimistic proposal generation.
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
    ) -> OptBlockData;

    /// Called when proxy blocks are ordered (committed by the BFT commit rule).
    ///
    /// With leader-driven cutting, the hook unconditionally forwards ALL ordered
    /// blocks to primary consensus. The primary leader decides where to cut.
    ///
    /// `committed_blocks` are the blocks committed in this round, in order.
    async fn on_ordered_blocks(&self, committed_blocks: Vec<Arc<Block>>);
}

// =============================================================================
// ProxyHooksImpl - Concrete implementation of ProxyConsensusHooks
// =============================================================================

/// Concrete implementation of ProxyConsensusHooks for the proxy RoundManager.
///
/// This struct bridges the proxy RoundManager (running standard Aptos BFT) with
/// primary consensus by unconditionally forwarding ordered proxy blocks to the
/// primary via channel + network broadcast.
pub struct ProxyHooksImpl {
    /// Channel to send OrderedProxyBlocksMsg to primary RoundManager.
    ordered_blocks_tx: tokio::sync::mpsc::UnboundedSender<ProxyToPrimaryEvent>,
    /// Network sender for broadcasting ordered blocks to all primaries.
    /// None only in unit tests where we don't have a real network stack.
    network: Option<Arc<crate::network::NetworkSender>>,
    /// This validator's identity. Used to gate network broadcast: only the
    /// proposer of the last block broadcasts to remote primaries.
    self_author: Author,
}

impl ProxyHooksImpl {
    pub fn new(
        ordered_blocks_tx: tokio::sync::mpsc::UnboundedSender<ProxyToPrimaryEvent>,
        network: Arc<crate::network::NetworkSender>,
        self_author: Author,
    ) -> Self {
        Self {
            ordered_blocks_tx,
            network: Some(network),
            self_author,
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
    ) -> BlockData {
        // Proxy uses round-robin leader election, so failed_authors should always
        // be empty. If it's not, the proposer election is misconfigured.
        if !failed_authors.is_empty() {
            warn!(
                round = round,
                num_failed = failed_authors.len(),
                "ProxyHooksImpl: unexpected non-empty failed_authors in proxy block"
            );
        }

        let payload_txns = payload.len();
        info!(
            round = round,
            payload_txns = payload_txns,
            num_vtxns = validator_txns.len(),
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
    ) -> OptBlockData {
        info!(
            round = round,
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
        )
    }

    async fn on_ordered_blocks(&self, committed_blocks: Vec<Arc<Block>>) {
        if committed_blocks.is_empty() {
            return;
        }

        // Collect blocks sorted by round (ascending = oldest-first), skip NIL blocks
        let mut blocks: Vec<Block> = committed_blocks
            .iter()
            .filter(|b| !b.is_nil_block())
            .map(|b| (**b).clone())
            .collect();

        if blocks.is_empty() {
            return;
        }

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

        proxy_metrics::PROXY_CONSENSUS_BLOCKS_ORDERED.inc_by(blocks.len() as u64);
        let opt_count = blocks.iter().filter(|b| b.is_opt_block()).count();
        if opt_count > 0 {
            proxy_metrics::PROXY_CONSENSUS_OPT_BLOCKS_ORDERED.inc_by(opt_count as u64);
        }

        // Diagnostic: log round range and txn counts
        let first_round = blocks.first().map(|b| b.round()).unwrap_or(0);
        let last_round = blocks.last().map(|b| b.round()).unwrap_or(0);
        let total_txns: usize = blocks.iter().map(|b| b.payload().map_or(0, |p| p.len())).sum();
        let empty_blocks = blocks
            .iter()
            .filter(|b| b.payload().map_or(true, |p| p.is_empty()))
            .count();
        let last_block_author = blocks.last().and_then(|b| b.author());

        info!(
            num_blocks = blocks.len(),
            first_proxy_round = first_round,
            last_proxy_round = last_round,
            total_txns = total_txns,
            empty_blocks = empty_blocks,
            opt_blocks = opt_count,
            "ProxyHooksImpl: forwarding ordered proxy blocks unconditionally"
        );

        // Construct the ordered proxy blocks message
        let ordered_msg = OrderedProxyBlocksMsg::new(blocks);

        // Only the proposer of the last block broadcasts to remote primaries.
        // All validators still send via local channel so their own primary gets blocks
        // instantly without waiting for a network round-trip.
        if last_block_author == Some(self.self_author) {
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
    ) -> Block {
        let block_data = BlockData::new_from_proxy(
            1, round,
            aptos_infallible::duration_since_epoch().as_micros() as u64,
            parent_qc,
            vec![],
            Payload::empty(true, true),
            signer.author(),
            vec![],
        );
        Block::new_proposal_from_block_data(block_data, signer).unwrap()
    }

    fn make_proxy_block_chain(
        signer: &ValidatorSigner,
        num_blocks: usize,
        start_round: Round,
    ) -> Vec<Block> {
        assert!(num_blocks > 0);
        let mut blocks = Vec::with_capacity(num_blocks);

        let genesis_qc = make_qc(1, 0);
        let first = make_proxy_block(signer, start_round, genesis_qc);
        blocks.push(first);

        for i in 1..num_blocks {
            let prev = &blocks[i - 1];
            let parent_qc = make_qc_for_block(1, prev.round(), prev.id());
            let block = make_proxy_block(signer, start_round + i as u64, parent_qc);
            blocks.push(block);
        }

        blocks
    }

    // ===================================================================
    // transform_proposal tests
    // ===================================================================

    #[test]
    fn test_transform_proposal_creates_proxy_block() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let signer = ValidatorSigner::from_int(0);

        // Create a minimal ProxyHooksImpl (without real network)
        let hooks = ProxyHooksImpl {
            ordered_blocks_tx: tx,
            network: None,
            self_author: signer.author(),
        };

        let qc = make_qc(1, 0);
        let block_data = hooks.transform_proposal(
            vec![],
            Payload::empty(true, true),
            signer.author(),
            vec![],
            1,
            1000,
            qc,
        );

        assert!(block_data.is_proxy_block());
    }

    // ===================================================================
    // on_ordered_blocks tests
    // ===================================================================

    #[tokio::test]
    async fn test_on_ordered_blocks_forwards_unconditionally() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let signer = ValidatorSigner::from_int(0);

        let hooks = ProxyHooksImpl {
            ordered_blocks_tx: tx,
            network: None,
            self_author: signer.author(),
        };

        let blocks = make_proxy_block_chain(&signer, 3, 1);
        let arc_blocks: Vec<Arc<Block>> = blocks.into_iter().map(Arc::new).collect();

        hooks.on_ordered_blocks(arc_blocks).await;

        // Should have received one message with all blocks
        let event = rx.try_recv().expect("Should receive ordered proxy blocks");
        match event {
            ProxyToPrimaryEvent::OrderedProxyBlocks(msg) => {
                assert_eq!(msg.proxy_blocks().len(), 3);
            },
        }
    }

    #[tokio::test]
    async fn test_on_ordered_blocks_skips_nil_blocks() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let signer = ValidatorSigner::from_int(0);

        let hooks = ProxyHooksImpl {
            ordered_blocks_tx: tx,
            network: None,
            self_author: signer.author(),
        };

        // Create a mix of proxy and nil blocks
        let proxy_block = make_proxy_block(&signer, 1, make_qc(1, 0));
        let nil_block = Block::new_nil(2, make_qc_for_block(1, 1, proxy_block.id()), vec![]);

        let arc_blocks: Vec<Arc<Block>> = vec![Arc::new(proxy_block), Arc::new(nil_block)];

        hooks.on_ordered_blocks(arc_blocks).await;

        // Should only forward the proxy block, not the nil block
        let event = rx.try_recv().expect("Should receive ordered proxy blocks");
        match event {
            ProxyToPrimaryEvent::OrderedProxyBlocks(msg) => {
                assert_eq!(msg.proxy_blocks().len(), 1);
            },
        }
    }

    #[tokio::test]
    async fn test_on_ordered_blocks_empty_input() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let signer = ValidatorSigner::from_int(0);

        let hooks = ProxyHooksImpl {
            ordered_blocks_tx: tx,
            network: None,
            self_author: signer.author(),
        };

        hooks.on_ordered_blocks(vec![]).await;

        // Should not have sent anything
        assert!(rx.try_recv().is_err());
    }
}
