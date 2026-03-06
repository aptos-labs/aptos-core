// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration types for primary consensus to consume proxy blocks.
//!
//! With leader-driven cutting, the primary buffers individual proxy blocks in a
//! BTreeMap and aggregates them at proposal time. `PrimaryBlockFromProxy` provides
//! helper methods for deterministic aggregation of payloads and validator txns.

use aptos_consensus_types::{
    block::Block,
    common::{Payload, Round},
};
use aptos_crypto::HashValue;
use aptos_logger::info;
use std::sync::Arc;

/// Aggregated proxy blocks ready to be included in a primary block.
///
/// Created from a range of blocks taken from the pending_proxy_blocks BTreeMap
/// during proposal generation (leader-driven cutting).
///
/// Key invariant: Given the same proxy block range, all primaries
/// MUST produce identical aggregation. This determinism is critical for consensus.
#[derive(Debug, Clone)]
pub struct PrimaryBlockFromProxy {
    /// Ordered proxy blocks (sorted by proxy round, ascending)
    proxy_blocks: Vec<Arc<Block>>,
    /// Round of the last proxy block included in this aggregation.
    last_proxy_round: Round,
    /// Block ID of the last proxy block included in this aggregation.
    last_proxy_block_id: HashValue,
    /// Aggregated payload hash (for deterministic ordering verification)
    aggregated_payload_hash: HashValue,
}

impl PrimaryBlockFromProxy {
    /// Create from a range of proxy blocks taken from the BTreeMap buffer.
    ///
    /// `blocks` must be non-empty and already sorted by ascending round.
    pub fn from_proxy_blocks(blocks: Vec<Arc<Block>>) -> Self {
        assert!(!blocks.is_empty(), "proxy blocks must be non-empty");

        let last_block = blocks.last().unwrap();
        let last_proxy_round = last_block.round();
        let last_proxy_block_id = last_block.id();

        let aggregated_payload_hash = Self::compute_aggregated_hash(&blocks);

        let first_round = blocks.first().map(|b| b.round()).unwrap_or(0);
        info!(
            "PrimaryBlockFromProxy: aggregated {} proxy blocks, rounds=[{}..{}], hash={}",
            blocks.len(),
            first_round,
            last_proxy_round,
            aggregated_payload_hash,
        );

        Self {
            proxy_blocks: blocks,
            last_proxy_round,
            last_proxy_block_id,
            aggregated_payload_hash,
        }
    }

    /// Compute a deterministic hash of all proxy block IDs.
    fn compute_aggregated_hash(proxy_blocks: &[Arc<Block>]) -> HashValue {
        let mut hasher = aptos_crypto::hash::DefaultHasher::new(b"AggregatedProxyBlocks");
        for block in proxy_blocks {
            hasher.update(&block.id().to_vec());
        }
        hasher.finish()
    }

    /// Get the proxy blocks.
    pub fn proxy_blocks(&self) -> &[Arc<Block>] {
        &self.proxy_blocks
    }

    /// Get the last proxy round in this aggregation.
    pub fn last_proxy_round(&self) -> Round {
        self.last_proxy_round
    }

    /// Get the last proxy block ID in this aggregation.
    pub fn last_proxy_block_id(&self) -> HashValue {
        self.last_proxy_block_id
    }

    /// Get the number of proxy blocks.
    pub fn num_blocks(&self) -> usize {
        self.proxy_blocks.len()
    }

    /// Get the aggregated payload hash for verification.
    pub fn aggregated_payload_hash(&self) -> HashValue {
        self.aggregated_payload_hash
    }

    /// Aggregate payloads from all proxy blocks.
    ///
    /// This combines the payloads deterministically so all primaries
    /// produce the same primary block payload. Uses `Payload::extend()`
    /// to merge payloads while preserving QuorumStore batch structure.
    pub fn aggregate_payloads(&self) -> Payload {
        let mut payloads: Vec<Payload> = self
            .proxy_blocks
            .iter()
            .filter_map(|b| b.payload().cloned())
            .filter(|p| !p.is_empty())
            .collect();

        if payloads.is_empty() {
            return Payload::empty(true, true);
        }

        let mut result = payloads.remove(0);
        for p in payloads {
            result = result.extend(p);
        }
        result
    }

    /// Get the total transaction count across all proxy blocks.
    pub fn total_txn_count(&self) -> usize {
        self.proxy_blocks
            .iter()
            .filter_map(|b| b.payload())
            .map(|p| p.len())
            .sum()
    }

    /// Check if any proxy block has validator transactions.
    pub fn has_validator_txns(&self) -> bool {
        self.proxy_blocks
            .iter()
            .any(|b| b.validator_txns().is_some_and(|txns| !txns.is_empty()))
    }

    /// Get all validator transactions from proxy blocks.
    pub fn validator_txns(&self) -> Vec<aptos_types::validator_txn::ValidatorTransaction> {
        self.proxy_blocks
            .iter()
            .filter_map(|b| b.validator_txns())
            .flatten()
            .cloned()
            .collect()
    }

    /// Aggregate validator transactions from all proxy blocks.
    pub fn aggregate_validator_txns(&self) -> Vec<aptos_types::validator_txn::ValidatorTransaction> {
        self.validator_txns()
    }

    /// Get the ID of the first proxy block.
    pub fn first_block_id(&self) -> HashValue {
        self.proxy_blocks
            .first()
            .map(|b| b.id())
            .unwrap_or_else(HashValue::zero)
    }

    /// Get the ID of the last proxy block.
    pub fn last_block_id(&self) -> HashValue {
        self.proxy_blocks
            .last()
            .map(|b| b.id())
            .unwrap_or_else(HashValue::zero)
    }

    /// Get the timestamp range of proxy blocks.
    pub fn timestamp_range(&self) -> (u64, u64) {
        let first_ts = self
            .proxy_blocks
            .first()
            .map(|b| b.timestamp_usecs())
            .unwrap_or(0);
        let last_ts = self
            .proxy_blocks
            .last()
            .map(|b| b.timestamp_usecs())
            .unwrap_or(0);
        (first_ts, last_ts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_consensus_types::{
        block_data::BlockData,
        common::Payload,
        quorum_cert::QuorumCert,
        vote_data::VoteData,
    };
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_info::BlockInfo,
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
        validator_signer::ValidatorSigner,
    };

    fn make_qc(epoch: u64, round: Round) -> QuorumCert {
        let block_info =
            BlockInfo::new(epoch, round, HashValue::random(), HashValue::random(), 0, 0, None);
        let vote_data = VoteData::new(block_info.clone(), block_info.clone());
        let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
        let li_sig = LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty());
        QuorumCert::new(vote_data, li_sig)
    }

    fn make_qc_for_block(epoch: u64, round: Round, block_id: HashValue) -> QuorumCert {
        let block_info =
            BlockInfo::new(epoch, round, block_id, HashValue::random(), 0, 0, None);
        let vote_data = VoteData::new(block_info.clone(), block_info.clone());
        let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
        let li_sig = LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty());
        QuorumCert::new(vote_data, li_sig)
    }

    fn make_proxy_block(
        signer: &ValidatorSigner,
        round: Round,
        parent_qc: QuorumCert,
    ) -> Block {
        let block_data = BlockData::new_from_proxy(
            1,
            round,
            aptos_infallible::duration_since_epoch().as_micros() as u64,
            parent_qc,
            vec![],
            Payload::empty(false, true),
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

    #[test]
    fn test_from_proxy_blocks_single_block() {
        let signer = ValidatorSigner::from_int(0);
        let block = make_proxy_block(&signer, 1, make_qc(1, 0));

        let result = PrimaryBlockFromProxy::from_proxy_blocks(vec![Arc::new(block.clone())]);

        assert_eq!(result.num_blocks(), 1);
        assert_eq!(result.last_proxy_round(), 1);
        assert_eq!(result.last_proxy_block_id(), block.id());
    }

    #[test]
    fn test_from_proxy_blocks_multiple_blocks() {
        let signer = ValidatorSigner::from_int(0);
        let blocks = make_proxy_block_chain(&signer, 3, 1);
        let last_id = blocks[2].id();

        let arc_blocks: Vec<Arc<Block>> = blocks.into_iter().map(Arc::new).collect();
        let result = PrimaryBlockFromProxy::from_proxy_blocks(arc_blocks);

        assert_eq!(result.num_blocks(), 3);
        assert_eq!(result.last_proxy_round(), 3);
        assert_eq!(result.last_proxy_block_id(), last_id);
    }

    #[test]
    fn test_aggregated_hash_deterministic() {
        let signer = ValidatorSigner::from_int(0);
        let block = make_proxy_block(&signer, 1, make_qc(1, 0));
        let arc_block = Arc::new(block);

        let result1 = PrimaryBlockFromProxy::from_proxy_blocks(vec![arc_block.clone()]);
        let result2 = PrimaryBlockFromProxy::from_proxy_blocks(vec![arc_block]);

        assert_eq!(
            result1.aggregated_payload_hash(),
            result2.aggregated_payload_hash(),
        );
    }

    #[test]
    fn test_aggregate_payloads_all_empty() {
        let signer = ValidatorSigner::from_int(0);
        let block = make_proxy_block(&signer, 1, make_qc(1, 0));

        let result = PrimaryBlockFromProxy::from_proxy_blocks(vec![Arc::new(block)]);
        let payload = result.aggregate_payloads();
        assert_eq!(payload.len(), 0);
    }

    #[test]
    fn test_accessors() {
        let signer = ValidatorSigner::from_int(0);
        let blocks = make_proxy_block_chain(&signer, 3, 1);
        let first_id = blocks[0].id();
        let last_id = blocks[2].id();
        let first_ts = blocks[0].timestamp_usecs();
        let last_ts = blocks[2].timestamp_usecs();

        let arc_blocks: Vec<Arc<Block>> = blocks.into_iter().map(Arc::new).collect();
        let result = PrimaryBlockFromProxy::from_proxy_blocks(arc_blocks);

        assert_eq!(result.first_block_id(), first_id);
        assert_eq!(result.last_block_id(), last_id);

        let (ts_start, ts_end) = result.timestamp_range();
        assert_eq!(ts_start, first_ts);
        assert_eq!(ts_end, last_ts);

        assert_eq!(result.total_txn_count(), 0);
        assert!(!result.has_validator_txns());
        assert!(result.validator_txns().is_empty());
    }
}
