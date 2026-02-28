// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration types for primary consensus to consume proxy blocks.
//!
//! This module provides:
//! - `PrimaryBlockFromProxy`: Aggregates ordered proxy blocks into primary block content
//! - Deterministic aggregation: All primaries produce identical blocks from same proxy blocks
//! - Verification: Ensures proxy blocks are valid and properly linked

use crate::proxy_error::ProxyConsensusError;
use aptos_consensus_types::{
    block::Block,
    common::{Payload, Round},
    primary_consensus_proof::PrimaryConsensusProof,
    proxy_messages::OrderedProxyBlocksMsg,
};
use aptos_crypto::HashValue;
use aptos_types::validator_verifier::ValidatorVerifier;
use aptos_logger::info;

/// Aggregated proxy blocks ready to be included in a primary block.
///
/// This structure is created from an `OrderedProxyBlocksMsg` and provides
/// the content needed to construct a primary block.
///
/// Key invariant: Given the same `OrderedProxyBlocksMsg`, all primaries
/// MUST produce identical `PrimaryBlockFromProxy` and therefore identical
/// primary blocks. This determinism is critical for consensus.
#[derive(Debug, Clone)]
pub struct PrimaryBlockFromProxy {
    /// Ordered proxy blocks (sorted by proxy round)
    proxy_blocks: Vec<Block>,
    /// Primary consensus proof (QC or TC) that "cut" these proxy blocks
    primary_proof: PrimaryConsensusProof,
    /// Aggregated payload hash (for deterministic ordering)
    aggregated_payload_hash: HashValue,
}

impl PrimaryBlockFromProxy {
    /// Create a new `PrimaryBlockFromProxy` from an ordered message.
    ///
    /// This constructor validates the message structure but does not
    /// verify cryptographic signatures (that's done by `verify`).
    pub fn from_ordered_msg(
        msg: OrderedProxyBlocksMsg,
    ) -> Result<Self, ProxyConsensusError> {
        let proxy_blocks = msg.proxy_blocks().to_vec();
        let primary_proof = msg.primary_proof().clone();

        // Verify non-empty
        if proxy_blocks.is_empty() {
            return Err(ProxyConsensusError::InvalidProxyBlock(
                "OrderedProxyBlocksMsg must contain at least one proxy block".into(),
            ));
        }

        // Verify blocks are in strictly ascending round order
        for i in 1..proxy_blocks.len() {
            if proxy_blocks[i].round() <= proxy_blocks[i - 1].round() {
                return Err(ProxyConsensusError::InvalidProxyBlock(format!(
                    "Proxy blocks not in ascending round order: block {} round {} <= block {} round {}",
                    i,
                    proxy_blocks[i].round(),
                    i - 1,
                    proxy_blocks[i - 1].round(),
                )));
            }
        }

        // Verify all blocks are proxy blocks with non-decreasing last_primary_proof_round.
        // A batch may span multiple cutting points (TC gaps in primary), so
        // last_primary_proof_round values increase at intermediate cutting points.
        let mut prev_lppr = 0;
        for (i, block) in proxy_blocks.iter().enumerate() {
            let block_lppr = block.block_data().last_primary_proof_round().ok_or_else(|| {
                ProxyConsensusError::InvalidProxyBlock(format!(
                    "Block {} is not a proxy block (missing last_primary_proof_round)",
                    i
                ))
            })?;
            if block_lppr < prev_lppr {
                return Err(ProxyConsensusError::InvalidPrimaryRound {
                    expected: prev_lppr,
                    got: block_lppr,
                });
            }
            prev_lppr = block_lppr;
        }

        // Verify the last block's last_primary_proof_round matches the primary proof round.
        // The cutting block should have last_primary_proof_round == proof.round.
        if prev_lppr != primary_proof.proof_round() {
            return Err(ProxyConsensusError::InvalidPrimaryRound {
                expected: primary_proof.proof_round(),
                got: prev_lppr,
            });
        }

        // Verify proxy blocks have empty failed_authors (proxy uses round-robin)
        for (i, block) in proxy_blocks.iter().enumerate() {
            if block.block_data().failed_authors().is_some_and(|fa| !fa.is_empty()) {
                return Err(ProxyConsensusError::InvalidProxyBlock(format!(
                    "Proxy block {} has non-empty failed_authors, expected empty (round-robin)",
                    i,
                )));
            }
        }

        // Verify blocks are properly linked
        for i in 1..proxy_blocks.len() {
            if proxy_blocks[i].parent_id() != proxy_blocks[i - 1].id() {
                return Err(ProxyConsensusError::InvalidProxyBlock(format!(
                    "Proxy blocks not properly linked: block {} parent {} != block {} id {}",
                    i,
                    proxy_blocks[i].parent_id(),
                    i - 1,
                    proxy_blocks[i - 1].id(),
                )));
            }
        }

        // Verify last block has primary proof attached
        let last_block = proxy_blocks.last().unwrap();
        if last_block.block_data().primary_proof().is_none() {
            return Err(ProxyConsensusError::InvalidProxyBlock(
                "Last proxy block must have primary proof attached".into(),
            ));
        }

        // Compute deterministic aggregated payload hash
        let aggregated_payload_hash = Self::compute_aggregated_hash(&proxy_blocks);

        let primary_round = primary_proof.proof_round() + 1;
        let first_round = proxy_blocks.first().map(|b| b.round()).unwrap_or(0);
        let last_round = proxy_blocks.last().map(|b| b.round()).unwrap_or(0);
        info!(
            "PrimaryBlockFromProxy: parsed {} proxy blocks for primary_round={}, \
             proxy_rounds=[{}..{}], proof_round={}, proof_type={}, hash={}",
            proxy_blocks.len(),
            primary_round,
            first_round,
            last_round,
            primary_proof.proof_round(),
            if primary_proof.is_qc() { "QC" } else { "TC" },
            aggregated_payload_hash,
        );

        Ok(Self {
            proxy_blocks,
            primary_proof,
            aggregated_payload_hash,
        })
    }

    /// Compute a deterministic hash of all proxy block payloads.
    ///
    /// This ensures all primaries can verify they have the same content.
    fn compute_aggregated_hash(proxy_blocks: &[Block]) -> HashValue {
        let mut hasher = aptos_crypto::hash::DefaultHasher::new(b"AggregatedProxyBlocks");
        for block in proxy_blocks {
            hasher.update(&block.id().to_vec());
        }
        hasher.finish()
    }

    /// Verify the proxy blocks have valid signatures.
    ///
    /// Only the last proxy block is fully verified. Its QC certifies the
    /// previous block, which transitively guarantees chain integrity (a
    /// quorum of validators voted on each ancestor). This avoids O(N)
    /// crypto work per batch and keeps the primary responsive.
    ///
    /// The primary proof (QC/TC from primary consensus) is always verified.
    pub fn verify(
        &self,
        proxy_verifier: &ValidatorVerifier,
        primary_verifier: &ValidatorVerifier,
    ) -> Result<(), ProxyConsensusError> {
        // Verify only the last proxy block's signature. Its QC covers the
        // previous block, so the entire chain is transitively authenticated.
        let last_block = self.proxy_blocks.last().ok_or_else(|| {
            ProxyConsensusError::InvalidProxyBlock("No proxy blocks to verify".into())
        })?;

        // The last block must be the cutting-point block with primary proof attached.
        if last_block.block_data().primary_proof().is_none() {
            return Err(ProxyConsensusError::InvalidProxyBlock(
                "Last proxy block must have primary proof attached".into(),
            ));
        }

        last_block
            .validate_proxy_signature(proxy_verifier, primary_verifier)
            .map_err(|e| ProxyConsensusError::InvalidProxyBlock(e.to_string()))?;

        // Verify primary proof using full verifier
        // (primary QC/TC is signed by all N validators, not just proxy subset)
        self.primary_proof
            .verify(primary_verifier)
            .map_err(|e| ProxyConsensusError::InvalidProxyBlock(e.to_string()))?;

        Ok(())
    }

    /// Get the proxy blocks.
    pub fn proxy_blocks(&self) -> &[Block] {
        &self.proxy_blocks
    }

    /// Get the primary round (derived from proof.round + 1).
    pub fn primary_round(&self) -> Round {
        self.primary_proof.proof_round() + 1
    }

    /// Get the primary consensus proof (QC or TC).
    pub fn primary_proof(&self) -> &PrimaryConsensusProof {
        &self.primary_proof
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
    ///
    /// Alias for `validator_txns()` — named for consistency with `aggregate_payloads()`.
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

/// Builder for creating proxy block aggregations during testing.
#[cfg(any(test, feature = "fuzzing"))]
pub struct PrimaryBlockFromProxyBuilder {
    proxy_blocks: Vec<Block>,
    primary_proof: Option<PrimaryConsensusProof>,
}

#[cfg(any(test, feature = "fuzzing"))]
impl PrimaryBlockFromProxyBuilder {
    pub fn new() -> Self {
        Self {
            proxy_blocks: Vec::new(),
            primary_proof: None,
        }
    }

    pub fn with_proxy_block(mut self, block: Block) -> Self {
        self.proxy_blocks.push(block);
        self
    }

    pub fn with_primary_proof(mut self, proof: PrimaryConsensusProof) -> Self {
        self.primary_proof = Some(proof);
        self
    }

    pub fn build(self) -> Result<PrimaryBlockFromProxy, ProxyConsensusError> {
        let primary_proof = self.primary_proof.ok_or_else(|| {
            ProxyConsensusError::InvalidProxyBlock("Primary proof required".into())
        })?;

        let aggregated_payload_hash =
            PrimaryBlockFromProxy::compute_aggregated_hash(&self.proxy_blocks);

        Ok(PrimaryBlockFromProxy {
            proxy_blocks: self.proxy_blocks,
            primary_proof,
            aggregated_payload_hash,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aptos_consensus_types::{block_data::BlockData, quorum_cert::QuorumCert, vote_data::VoteData};
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

    /// Create a QC that certifies a specific block (by block_id and round).
    fn make_qc_for_block(epoch: u64, round: Round, block_id: HashValue) -> QuorumCert {
        let block_info =
            BlockInfo::new(epoch, round, block_id, HashValue::random(), 0, 0, None);
        let vote_data = VoteData::new(block_info.clone(), block_info.clone());
        let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
        let li_sig = LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty());
        QuorumCert::new(vote_data, li_sig)
    }

    /// Create a signed proxy Block.
    fn make_proxy_block(
        signer: &ValidatorSigner,
        round: Round,
        parent_qc: QuorumCert,
        last_primary_proof_round: Round,
        primary_proof: Option<PrimaryConsensusProof>,
    ) -> Block {
        let block_data = BlockData::new_from_proxy(
            1, // epoch
            round,
            aptos_infallible::duration_since_epoch().as_micros() as u64,
            parent_qc,
            vec![],                    // validator_txns
            Payload::empty(false, true), // payload
            signer.author(),
            vec![],                    // failed_authors
            last_primary_proof_round,
            primary_proof,
        );
        Block::new_proposal_from_block_data(block_data, signer).unwrap()
    }

    /// Create a chain of linked proxy blocks. Only the last block gets primary_proof attached.
    fn make_proxy_block_chain(
        signer: &ValidatorSigner,
        num_blocks: usize,
        start_round: Round,
        last_primary_proof_round: Round,
        primary_qc: Option<QuorumCert>,
    ) -> Vec<Block> {
        assert!(num_blocks > 0);
        let mut blocks = Vec::with_capacity(num_blocks);

        // First block uses a genesis QC
        let genesis_qc = make_qc(1, 0);
        let is_last = num_blocks == 1;
        let first_proof = if is_last { primary_qc.as_ref().map(|qc| PrimaryConsensusProof::QC(qc.clone())) } else { None };
        let first = make_proxy_block(signer, start_round, genesis_qc, last_primary_proof_round, first_proof);
        blocks.push(first);

        for i in 1..num_blocks {
            let prev = &blocks[i - 1];
            let parent_qc = make_qc_for_block(1, prev.round(), prev.id());
            let is_last = i == num_blocks - 1;
            let proof = if is_last { primary_qc.as_ref().map(|qc| PrimaryConsensusProof::QC(qc.clone())) } else { None };
            let block = make_proxy_block(
                signer,
                start_round + i as u64,
                parent_qc,
                last_primary_proof_round,
                proof,
            );
            blocks.push(block);
        }

        blocks
    }

    // =========================================================================
    // Empty / basic rejection tests
    // =========================================================================

    #[test]
    fn test_empty_proxy_blocks_rejected() {
        let primary_qc = make_qc(1, 5);
        let msg = OrderedProxyBlocksMsg::new(vec![], PrimaryConsensusProof::QC(primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_err());
        assert!(format!("{:?}", result.unwrap_err()).contains("at least one"));
    }

    #[test]
    fn test_last_block_lppr_mismatch_proof_round_rejected() {
        let signer = ValidatorSigner::from_int(0);
        // Block claims lppr=10, but proof.round=2 → mismatch → rejected
        let primary_qc = make_qc(1, 2);
        let block = make_proxy_block(
            &signer, 1, make_qc(1, 0),
            10, // mismatched: block says lppr=10, proof says round=2
            Some(PrimaryConsensusProof::QC(primary_qc.clone())),
        );
        let msg = OrderedProxyBlocksMsg::new(vec![block], PrimaryConsensusProof::QC(primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_err());
    }

    #[test]
    fn test_tc_gap_lppr_matches_proof_round_accepted() {
        // TC gap case: proof.round=5 (skipped rounds via TC).
        // Block's lppr = proof.round = 5 → should succeed.
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 5;
        let primary_qc = make_qc(1, proof_round);
        let block = make_proxy_block(
            &signer, 1, make_qc(1, 0),
            proof_round, // lppr = proof.round
            Some(PrimaryConsensusProof::QC(primary_qc.clone())),
        );
        let msg = OrderedProxyBlocksMsg::new(vec![block], PrimaryConsensusProof::QC(primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().primary_round(), proof_round + 1);
    }

    // =========================================================================
    // Happy path tests
    // =========================================================================

    #[test]
    fn test_from_ordered_msg_single_block() {
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let msg_primary_qc = make_qc(1, proof_round);

        let block = make_proxy_block(
            &signer,
            1,
            make_qc(1, 0),
            proof_round, // last_primary_proof_round = proof.round
            Some(PrimaryConsensusProof::QC(msg_primary_qc.clone())),
        );

        let msg = OrderedProxyBlocksMsg::new(vec![block], PrimaryConsensusProof::QC(msg_primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg).unwrap();

        assert_eq!(result.num_blocks(), 1);
        assert_eq!(result.primary_round(), proof_round + 1);
        assert_eq!(result.proxy_blocks().len(), 1);
    }

    #[test]
    fn test_from_ordered_msg_multiple_linked_blocks() {
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let msg_primary_qc = make_qc(1, proof_round);

        let blocks =
            make_proxy_block_chain(&signer, 3, 1, proof_round, Some(msg_primary_qc.clone()));

        let msg = OrderedProxyBlocksMsg::new(blocks.clone(), PrimaryConsensusProof::QC(msg_primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg).unwrap();

        assert_eq!(result.num_blocks(), 3);
        assert_eq!(result.primary_round(), proof_round + 1);
        // Verify block order is preserved
        assert_eq!(result.proxy_blocks()[0].id(), blocks[0].id());
        assert_eq!(result.proxy_blocks()[1].id(), blocks[1].id());
        assert_eq!(result.proxy_blocks()[2].id(), blocks[2].id());
    }

    // =========================================================================
    // Validation error tests
    // =========================================================================

    #[test]
    fn test_from_ordered_msg_unlinked_blocks_rejected() {
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let msg_primary_qc = make_qc(1, proof_round);

        // Create two independent blocks (not linked by parent)
        let block1 = make_proxy_block(&signer, 1, make_qc(1, 0), proof_round, None);
        let block2 = make_proxy_block(
            &signer,
            2,
            make_qc(1, 0), // NOT referencing block1
            proof_round,
            Some(PrimaryConsensusProof::QC(msg_primary_qc.clone())),
        );

        let msg =
            OrderedProxyBlocksMsg::new(vec![block1, block2], PrimaryConsensusProof::QC(msg_primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("not properly linked"),
            "Expected 'not properly linked' error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_from_ordered_msg_missing_primary_qc_on_last_block() {
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let msg_primary_qc = make_qc(1, proof_round);

        // Single block with NO primary_qc attached
        let block = make_proxy_block(&signer, 1, make_qc(1, 0), proof_round, None);

        let msg = OrderedProxyBlocksMsg::new(vec![block], PrimaryConsensusProof::QC(msg_primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("primary proof"),
            "Expected primary proof error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_from_ordered_msg_non_proxy_block_rejected() {
        let signer = ValidatorSigner::from_int(0);
        let msg_primary_qc = make_qc(1, 1);

        // Create a normal (non-proxy) block
        let normal_block = Block::new_proposal(
            Payload::empty(false, true),
            1,
            aptos_infallible::duration_since_epoch().as_micros() as u64,
            make_qc(1, 0),
            &signer,
            vec![],
        )
        .unwrap();

        let msg = OrderedProxyBlocksMsg::new(vec![normal_block], PrimaryConsensusProof::QC(msg_primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_err());
        let err_msg = format!("{:?}", result.unwrap_err());
        assert!(
            err_msg.contains("not a proxy block"),
            "Expected 'not a proxy block' error, got: {}",
            err_msg
        );
    }

    // =========================================================================
    // Safety rule enforcement tests
    // =========================================================================

    #[test]
    fn test_non_decreasing_lppr_across_multi_block_batch() {
        // Batch spanning multiple cutting points (TC gaps): blocks have varying lppr
        // block1: lppr=1, block2: lppr=1, block3: lppr=3 (intermediate cut), block4: lppr=5 (final cut)
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 5;
        let primary_qc = make_qc(1, proof_round);

        let block1 = make_proxy_block(&signer, 1, make_qc(1, 0), 1, None);
        let block2 = make_proxy_block(
            &signer, 2, make_qc_for_block(1, 1, block1.id()), 1, None,
        );
        let block3 = make_proxy_block(
            &signer, 3, make_qc_for_block(1, 2, block2.id()),
            3, // lppr increases at intermediate cutting point
            Some(PrimaryConsensusProof::QC(make_qc(1, 3))),
        );
        let block4 = make_proxy_block(
            &signer, 4, make_qc_for_block(1, 3, block3.id()),
            proof_round, // lppr = proof.round = 5 at final cutting point
            Some(PrimaryConsensusProof::QC(primary_qc.clone())),
        );

        let msg = OrderedProxyBlocksMsg::new(
            vec![block1, block2, block3, block4],
            PrimaryConsensusProof::QC(primary_qc),
        );
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_ok(), "Non-decreasing lppr [1,1,3,5] should be accepted");
        assert_eq!(result.unwrap().primary_round(), proof_round + 1);
    }

    #[test]
    fn test_decreasing_lppr_rejected() {
        // Block sequence where lppr decreases: block1 lppr=5, block2 lppr=3 → rejected
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 3;
        let primary_qc = make_qc(1, proof_round);

        let block1 = make_proxy_block(&signer, 1, make_qc(1, 0), 5, None);
        let block2 = make_proxy_block(
            &signer, 2, make_qc_for_block(1, 1, block1.id()),
            3, // lppr DECREASES from 5 to 3 → violation
            Some(PrimaryConsensusProof::QC(primary_qc.clone())),
        );

        let msg = OrderedProxyBlocksMsg::new(
            vec![block1, block2],
            PrimaryConsensusProof::QC(primary_qc),
        );
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_err(), "Decreasing lppr should be rejected");
    }

    #[test]
    fn test_tc_proof_accepted() {
        // Verify TC proof path works (not just QC)
        use aptos_consensus_types::timeout_2chain::{TwoChainTimeout, TwoChainTimeoutCertificate};

        let signer = ValidatorSigner::from_int(0);
        let tc_round = 5;
        let timeout = TwoChainTimeout::new(1, tc_round, QuorumCert::dummy());
        let tc = TwoChainTimeoutCertificate::new(timeout);
        let tc_proof = PrimaryConsensusProof::TC(tc);

        let block = make_proxy_block(
            &signer, 1, make_qc(1, 0),
            tc_round, // lppr = tc.round
            Some(tc_proof.clone()),
        );

        let msg = OrderedProxyBlocksMsg::new(vec![block], tc_proof);
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_ok(), "TC proof should be accepted");
        let pbfp = result.unwrap();
        assert_eq!(pbfp.primary_round(), tc_round + 1);
        assert!(pbfp.primary_proof().is_tc());
    }

    #[test]
    fn test_failed_authors_non_empty_rejected() {
        // Proxy blocks must have empty failed_authors (round-robin scheduling)
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let primary_qc = make_qc(1, proof_round);

        // Create block with non-empty failed_authors
        let block_data = BlockData::new_from_proxy(
            1, 1,
            aptos_infallible::duration_since_epoch().as_micros() as u64,
            make_qc(1, 0),
            vec![],
            Payload::empty(false, true),
            signer.author(),
            vec![(1, signer.author())], // non-empty failed_authors
            proof_round,
            Some(PrimaryConsensusProof::QC(primary_qc.clone())),
        );
        let block = Block::new_proposal_from_block_data(block_data, &signer).unwrap();

        let msg = OrderedProxyBlocksMsg::new(vec![block], PrimaryConsensusProof::QC(primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_err(), "Non-empty failed_authors should be rejected");
        assert!(format!("{:?}", result.unwrap_err()).contains("failed_authors"));
    }

    #[test]
    fn test_non_ascending_round_order_rejected() {
        // Blocks not in strictly ascending round order should be rejected
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let primary_qc = make_qc(1, proof_round);

        // block1 round=2, block2 round=1 → not ascending
        let block1 = make_proxy_block(&signer, 2, make_qc(1, 0), proof_round, None);
        let block2 = make_proxy_block(
            &signer, 1, make_qc_for_block(1, 2, block1.id()),
            proof_round,
            Some(PrimaryConsensusProof::QC(primary_qc.clone())),
        );

        let msg = OrderedProxyBlocksMsg::new(
            vec![block1, block2],
            PrimaryConsensusProof::QC(primary_qc),
        );
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_err(), "Non-ascending round order should be rejected");
        assert!(format!("{:?}", result.unwrap_err()).contains("ascending round"));
    }

    #[test]
    fn test_equal_rounds_rejected() {
        // Blocks with equal rounds should be rejected (must be strictly ascending)
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let primary_qc = make_qc(1, proof_round);

        let block1 = make_proxy_block(&signer, 1, make_qc(1, 0), proof_round, None);
        let block2 = make_proxy_block(
            &signer, 1, make_qc_for_block(1, 1, block1.id()), // same round=1
            proof_round,
            Some(PrimaryConsensusProof::QC(primary_qc.clone())),
        );

        let msg = OrderedProxyBlocksMsg::new(
            vec![block1, block2],
            PrimaryConsensusProof::QC(primary_qc),
        );
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_err(), "Equal rounds should be rejected");
    }

    #[test]
    fn test_last_block_without_proof_rejected() {
        // All blocks present but none has primary_proof → last block has no proof
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let primary_qc = make_qc(1, proof_round);

        let block1 = make_proxy_block(&signer, 1, make_qc(1, 0), proof_round, None);
        let block2 = make_proxy_block(
            &signer, 2, make_qc_for_block(1, 1, block1.id()),
            proof_round, None, // no proof on last block
        );

        let msg = OrderedProxyBlocksMsg::new(
            vec![block1, block2],
            PrimaryConsensusProof::QC(primary_qc),
        );
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg);
        assert!(result.is_err(), "Last block without proof should be rejected");
        assert!(format!("{:?}", result.unwrap_err()).contains("primary proof"));
    }

    #[test]
    fn test_primary_round_derivation() {
        // primary_round() = proof.proof_round() + 1
        let signer = ValidatorSigner::from_int(0);
        for proof_round in [0, 1, 5, 100] {
            let primary_qc = make_qc(1, proof_round);
            let block = make_proxy_block(
                &signer, 1, make_qc(1, 0),
                proof_round,
                Some(PrimaryConsensusProof::QC(primary_qc.clone())),
            );
            let msg = OrderedProxyBlocksMsg::new(vec![block], PrimaryConsensusProof::QC(primary_qc));
            let result = PrimaryBlockFromProxy::from_ordered_msg(msg).unwrap();
            assert_eq!(
                result.primary_round(), proof_round + 1,
                "primary_round() must equal proof.round + 1 for proof_round={}",
                proof_round,
            );
        }
    }

    // =========================================================================
    // Determinism and accessor tests
    // =========================================================================

    #[test]
    fn test_aggregated_hash_deterministic() {
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let msg_primary_qc = make_qc(1, proof_round);

        let block = make_proxy_block(
            &signer,
            1,
            make_qc(1, 0),
            proof_round,
            Some(PrimaryConsensusProof::QC(msg_primary_qc.clone())),
        );

        // Create two PrimaryBlockFromProxy from the same block
        let msg1 =
            OrderedProxyBlocksMsg::new(vec![block.clone()], PrimaryConsensusProof::QC(msg_primary_qc.clone()));
        let msg2 = OrderedProxyBlocksMsg::new(vec![block], PrimaryConsensusProof::QC(msg_primary_qc));

        let result1 = PrimaryBlockFromProxy::from_ordered_msg(msg1).unwrap();
        let result2 = PrimaryBlockFromProxy::from_ordered_msg(msg2).unwrap();

        assert_eq!(
            result1.aggregated_payload_hash(),
            result2.aggregated_payload_hash(),
            "Same blocks should produce the same aggregated hash"
        );
    }

    #[test]
    fn test_aggregated_hash_differs_for_different_blocks() {
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let msg_primary_qc = make_qc(1, proof_round);

        let block1 = make_proxy_block(
            &signer,
            1,
            make_qc(1, 0),
            proof_round,
            Some(PrimaryConsensusProof::QC(msg_primary_qc.clone())),
        );
        let block2 = make_proxy_block(
            &signer,
            2,
            make_qc(1, 0),
            proof_round,
            Some(PrimaryConsensusProof::QC(msg_primary_qc.clone())),
        );

        let msg1 =
            OrderedProxyBlocksMsg::new(vec![block1], PrimaryConsensusProof::QC(msg_primary_qc.clone()));
        let msg2 = OrderedProxyBlocksMsg::new(vec![block2], PrimaryConsensusProof::QC(msg_primary_qc));

        let result1 = PrimaryBlockFromProxy::from_ordered_msg(msg1).unwrap();
        let result2 = PrimaryBlockFromProxy::from_ordered_msg(msg2).unwrap();

        assert_ne!(
            result1.aggregated_payload_hash(),
            result2.aggregated_payload_hash(),
            "Different blocks should produce different aggregated hashes"
        );
    }

    #[test]
    fn test_aggregate_payloads_all_empty() {
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let msg_primary_qc = make_qc(1, proof_round);

        let block = make_proxy_block(
            &signer,
            1,
            make_qc(1, 0),
            proof_round,
            Some(PrimaryConsensusProof::QC(msg_primary_qc.clone())),
        );

        let msg = OrderedProxyBlocksMsg::new(vec![block], PrimaryConsensusProof::QC(msg_primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg).unwrap();

        // All proxy blocks have empty payloads → aggregate returns empty
        let payload = result.aggregate_payloads();
        assert_eq!(payload.len(), 0);
    }

    #[test]
    fn test_accessors() {
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let msg_primary_qc = make_qc(1, proof_round);

        let blocks =
            make_proxy_block_chain(&signer, 3, 1, proof_round, Some(msg_primary_qc.clone()));
        let first_id = blocks[0].id();
        let last_id = blocks[2].id();
        let first_ts = blocks[0].timestamp_usecs();
        let last_ts = blocks[2].timestamp_usecs();

        let msg = OrderedProxyBlocksMsg::new(blocks, PrimaryConsensusProof::QC(msg_primary_qc));
        let result = PrimaryBlockFromProxy::from_ordered_msg(msg).unwrap();

        assert_eq!(result.first_block_id(), first_id);
        assert_eq!(result.last_block_id(), last_id);

        let (ts_start, ts_end) = result.timestamp_range();
        assert_eq!(ts_start, first_ts);
        assert_eq!(ts_end, last_ts);

        // Empty payloads → total_txn_count = 0
        assert_eq!(result.total_txn_count(), 0);
        assert!(!result.has_validator_txns());
        assert!(result.validator_txns().is_empty());
    }

    // =========================================================================
    // Builder tests
    // =========================================================================

    #[test]
    fn test_builder_with_blocks_and_qc() {
        let signer = ValidatorSigner::from_int(0);
        let proof_round = 1;
        let msg_primary_qc = make_qc(1, proof_round);

        let block = make_proxy_block(
            &signer,
            1,
            make_qc(1, 0),
            proof_round,
            Some(PrimaryConsensusProof::QC(msg_primary_qc.clone())),
        );

        let result = PrimaryBlockFromProxyBuilder::new()
            .with_proxy_block(block)
            .with_primary_proof(PrimaryConsensusProof::QC(msg_primary_qc))
            .build();

        assert!(result.is_ok());
        let pbfp = result.unwrap();
        assert_eq!(pbfp.num_blocks(), 1);
        assert_eq!(pbfp.primary_round(), proof_round + 1);
    }

    #[test]
    fn test_builder_without_qc_fails() {
        let signer = ValidatorSigner::from_int(0);

        let block = make_proxy_block(
            &signer,
            1,
            make_qc(1, 0),
            1, // last_primary_proof_round
            None,
        );

        let result = PrimaryBlockFromProxyBuilder::new()
            .with_proxy_block(block)
            .build();

        assert!(result.is_err());
    }
}
