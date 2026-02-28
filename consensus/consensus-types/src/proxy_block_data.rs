// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Proxy block data types for proxy primary consensus.
//!
//! Proxy blocks are produced by a subset of validators (proxies) running fast local consensus.
//! They are aggregated into primary blocks by the full validator set.

use crate::{
    common::{Author, Payload, Round},
    primary_consensus_proof::PrimaryConsensusProof,
    quorum_cert::QuorumCert,
};
use anyhow::ensure;
use aptos_crypto::HashValue;
use aptos_crypto_derive::CryptoHasher;
use aptos_infallible::duration_since_epoch;
use aptos_types::{block_info::BlockInfo, validator_txn::ValidatorTransaction};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    ops::Deref,
};

/// Optimistic proxy block data (1 message delay).
///
/// Similar to OptBlockData but with additional fields for primary consensus linkage:
/// - `last_primary_proof_round`: Round of the most recent primary proof (QC/TC) in this block's ancestry
/// - `primary_proof`: Attached at cutting points when a new primary proof is available
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, CryptoHasher)]
pub struct OptProxyBlockData {
    pub epoch: u64,
    pub round: Round,
    pub timestamp_usecs: u64,
    pub parent: BlockInfo,
    pub block_body: OptProxyBlockBody,
}

/// Versioned body for optimistic proxy blocks.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum OptProxyBlockBody {
    V0 {
        /// Validator transactions (aggregated DKG/JWK submissions)
        validator_txns: Vec<ValidatorTransaction>,
        /// User transaction payload
        payload: Payload,
        /// Author of this proxy block
        author: Author,
        /// QC on grandparent proxy block (for optimistic path)
        grandparent_qc: QuorumCert,
        /// Round of the most recent primary proof (QC/TC) in this block's ancestry
        last_primary_proof_round: Round,
        /// Primary consensus proof (QC or TC) attached at this cutting point
        primary_proof: Option<PrimaryConsensusProof>,
    },
}

impl OptProxyBlockData {
    pub fn new(
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        epoch: u64,
        round: Round,
        timestamp_usecs: u64,
        parent: BlockInfo,
        grandparent_qc: QuorumCert,
        last_primary_proof_round: Round,
        primary_proof: Option<PrimaryConsensusProof>,
    ) -> Self {
        Self {
            epoch,
            round,
            timestamp_usecs,
            parent,
            block_body: OptProxyBlockBody::V0 {
                validator_txns,
                payload,
                author,
                grandparent_qc,
                last_primary_proof_round,
                primary_proof,
            },
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.timestamp_usecs
    }

    pub fn parent_id(&self) -> HashValue {
        self.parent.id()
    }

    pub fn parent(&self) -> &BlockInfo {
        &self.parent
    }

    /// Verify the proxy block is well-formed.
    ///
    /// Checks:
    /// - Round relationships (grandparent -> parent -> this)
    /// - Epoch consistency
    /// - Timestamp monotonicity
    /// - last_primary_proof_round consistency with attached proof
    pub fn verify_well_formed(&self) -> anyhow::Result<()> {
        let parent = self.parent();
        let grandparent_qc = self.grandparent_qc().certified_block();

        // Standard optimistic block checks
        ensure!(
            grandparent_qc.round() + 1 == parent.round(),
            "Block's parent's round {} must be one more than grandparent's round {}",
            parent.round(),
            grandparent_qc.round(),
        );
        ensure!(
            parent.round() + 1 == self.round(),
            "Block's round {} must be one more than parent's round {}",
            self.round(),
            parent.round(),
        );
        ensure!(
            grandparent_qc.epoch() == self.epoch() && parent.epoch() == self.epoch(),
            "Block's parent and grandparent should be in the same epoch"
        );
        ensure!(
            !grandparent_qc.has_reconfiguration(),
            "Optimistic proposals are disallowed after the reconfiguration block"
        );

        self.payload().verify_epoch(self.epoch())?;

        ensure!(
            self.timestamp_usecs() > parent.timestamp_usecs()
                && parent.timestamp_usecs() > grandparent_qc.timestamp_usecs(),
            "Blocks must have strictly increasing timestamps"
        );

        let current_ts = duration_since_epoch();
        const TIMEBOUND: u64 = 300_000_000; // 5 minutes in microseconds
        ensure!(
            self.timestamp_usecs() <= (current_ts.as_micros() as u64).saturating_add(TIMEBOUND),
            "Blocks must not be too far in the future"
        );

        // Validate last_primary_proof_round consistency with attached proof.
        // If proof is attached, proof.round must be >= last_primary_proof_round
        // (it should equal last_primary_proof_round when set correctly by proposer).
        if let Some(ref primary_proof) = self.primary_proof() {
            ensure!(
                primary_proof.proof_round() >= self.last_primary_proof_round(),
                "Primary proof round {} must be >= last_primary_proof_round {}",
                primary_proof.proof_round(),
                self.last_primary_proof_round(),
            );
        }

        Ok(())
    }

    /// Returns true if this proxy block has a primary proof attached (cutting point).
    pub fn has_primary_proof(&self) -> bool {
        self.primary_proof().is_some()
    }
}

impl OptProxyBlockBody {
    pub fn author(&self) -> &Author {
        match self {
            OptProxyBlockBody::V0 { author, .. } => author,
        }
    }

    pub fn validator_txns(&self) -> &Vec<ValidatorTransaction> {
        match self {
            OptProxyBlockBody::V0 { validator_txns, .. } => validator_txns,
        }
    }

    pub fn payload(&self) -> &Payload {
        match self {
            OptProxyBlockBody::V0 { payload, .. } => payload,
        }
    }

    pub fn grandparent_qc(&self) -> &QuorumCert {
        match self {
            OptProxyBlockBody::V0 { grandparent_qc, .. } => grandparent_qc,
        }
    }

    pub fn last_primary_proof_round(&self) -> Round {
        match self {
            OptProxyBlockBody::V0 { last_primary_proof_round, .. } => *last_primary_proof_round,
        }
    }

    pub fn primary_proof(&self) -> Option<&PrimaryConsensusProof> {
        match self {
            OptProxyBlockBody::V0 { primary_proof, .. } => primary_proof.as_ref(),
        }
    }
}

impl Deref for OptProxyBlockData {
    type Target = OptProxyBlockBody;

    fn deref(&self) -> &Self::Target {
        &self.block_body
    }
}

impl Display for OptProxyBlockData {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "[proxy author: {}, epoch: {}, round: {:02}, last_primary_proof_round: {:02}, parent_id: {}, timestamp: {}, has_primary_proof: {}]",
            self.author(),
            self.epoch(),
            self.round(),
            self.last_primary_proof_round(),
            self.parent_id(),
            self.timestamp_usecs(),
            self.has_primary_proof(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vote_data::VoteData;
    use aptos_crypto::HashValue;
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        block_info::BlockInfo,
        ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    };

    fn make_block_info(epoch: u64, round: Round, timestamp: u64) -> BlockInfo {
        BlockInfo::new(
            epoch,
            round,
            HashValue::random(),
            HashValue::random(),
            0,
            timestamp,
            None,
        )
    }

    fn make_qc(epoch: u64, round: Round, timestamp: u64) -> QuorumCert {
        let block_info = make_block_info(epoch, round, timestamp);
        let vote_data = VoteData::new(block_info.clone(), block_info.clone());
        let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
        let li_sig = LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty());
        QuorumCert::new(vote_data, li_sig)
    }

    #[test]
    fn test_opt_proxy_block_data_creation() {
        let epoch = 1;
        let grandparent_qc = make_qc(epoch, 1, 1000);
        let parent = make_block_info(epoch, 2, 2000);

        let block = OptProxyBlockData::new(
            vec![],
            Payload::empty(false, true),
            Author::random(),
            epoch,
            3, // round
            3000,
            parent,
            grandparent_qc,
            1, // last_primary_proof_round
            None,
        );

        assert_eq!(block.epoch(), epoch);
        assert_eq!(block.round(), 3);
        assert_eq!(block.last_primary_proof_round(), 1);
        assert!(!block.has_primary_proof());
    }

    #[test]
    fn test_opt_proxy_block_data_with_primary_proof() {
        let epoch = 1;
        let grandparent_qc = make_qc(epoch, 1, 1000);
        let parent = make_block_info(epoch, 2, 2000);
        let primary_qc = make_qc(epoch, 1, 500); // QC for primary round 1

        let block = OptProxyBlockData::new(
            vec![],
            Payload::empty(false, true),
            Author::random(),
            epoch,
            3,
            3000,
            parent,
            grandparent_qc,
            1, // last_primary_proof_round = proof.round = 1
            Some(PrimaryConsensusProof::QC(primary_qc)),
        );

        assert!(block.has_primary_proof());
        assert_eq!(block.primary_proof().unwrap().proof_round(), 1);
        assert_eq!(block.last_primary_proof_round(), 1);
    }

    // =========================================================================
    // verify_well_formed() safety rule tests
    // =========================================================================

    /// Helper: create a well-formed OptProxyBlockData for verify_well_formed tests.
    fn make_well_formed_block(
        epoch: u64,
        grandparent_round: Round,
        parent_round: Round,
        round: Round,
        lppr: Round,
        primary_proof: Option<PrimaryConsensusProof>,
    ) -> OptProxyBlockData {
        OptProxyBlockData::new(
            vec![],
            Payload::empty(false, true),
            Author::random(),
            epoch,
            round,
            (round + 1) * 1000, // strictly increasing timestamps
            make_block_info(epoch, parent_round, parent_round * 1000),
            make_qc(epoch, grandparent_round, (grandparent_round.saturating_sub(1)) * 1000),
            lppr,
            primary_proof,
        )
    }

    #[test]
    fn test_verify_well_formed_happy_path_no_proof() {
        // grandparent=1, parent=2, block=3, lppr=0, no proof
        let block = make_well_formed_block(1, 1, 2, 3, 0, None);
        assert!(block.verify_well_formed().is_ok());
    }

    #[test]
    fn test_verify_well_formed_happy_path_with_proof() {
        // grandparent=1, parent=2, block=3, lppr=1, proof.round=1
        let proof = PrimaryConsensusProof::QC(make_qc(1, 1, 500));
        let block = make_well_formed_block(1, 1, 2, 3, 1, Some(proof));
        assert!(block.verify_well_formed().is_ok());
    }

    #[test]
    fn test_verify_well_formed_grandparent_parent_gap() {
        // grandparent=1, parent=3 → parent.round != grandparent.round + 1
        let block = make_well_formed_block(1, 1, 3, 4, 0, None);
        let result = block.verify_well_formed();
        assert!(result.is_err(), "Gap between grandparent and parent round should fail");
        assert!(result.unwrap_err().to_string().contains("one more than grandparent"));
    }

    #[test]
    fn test_verify_well_formed_parent_block_gap() {
        // grandparent=1, parent=2, block=4 → block.round != parent.round + 1
        let block = make_well_formed_block(1, 1, 2, 4, 0, None);
        let result = block.verify_well_formed();
        assert!(result.is_err(), "Gap between parent and block round should fail");
        assert!(result.unwrap_err().to_string().contains("one more than parent"));
    }

    #[test]
    fn test_verify_well_formed_epoch_mismatch_parent() {
        // Parent in different epoch
        let block = OptProxyBlockData::new(
            vec![],
            Payload::empty(false, true),
            Author::random(),
            1, // block epoch = 1
            3,
            3000,
            make_block_info(2, 2, 2000), // parent epoch = 2 → mismatch
            make_qc(1, 1, 500),
            0,
            None,
        );
        let result = block.verify_well_formed();
        assert!(result.is_err(), "Epoch mismatch with parent should fail");
        assert!(result.unwrap_err().to_string().contains("same epoch"));
    }

    #[test]
    fn test_verify_well_formed_epoch_mismatch_grandparent() {
        // Grandparent in different epoch
        let block = OptProxyBlockData::new(
            vec![],
            Payload::empty(false, true),
            Author::random(),
            1, // block epoch = 1
            3,
            3000,
            make_block_info(1, 2, 2000),
            make_qc(2, 1, 500), // grandparent epoch = 2 → mismatch
            0,
            None,
        );
        let result = block.verify_well_formed();
        assert!(result.is_err(), "Epoch mismatch with grandparent should fail");
    }

    #[test]
    fn test_verify_well_formed_timestamp_not_increasing() {
        // parent.timestamp >= block.timestamp → violation
        let block = OptProxyBlockData::new(
            vec![],
            Payload::empty(false, true),
            Author::random(),
            1,
            3,
            1000, // block timestamp = 1000
            make_block_info(1, 2, 2000), // parent timestamp = 2000 > block → violation
            make_qc(1, 1, 500),
            0,
            None,
        );
        let result = block.verify_well_formed();
        assert!(result.is_err(), "Non-increasing timestamp should fail");
        assert!(result.unwrap_err().to_string().contains("timestamp"));
    }

    #[test]
    fn test_verify_well_formed_proof_round_less_than_lppr() {
        // proof.round (1) < lppr (5) → violation
        let proof = PrimaryConsensusProof::QC(make_qc(1, 1, 500));
        let block = make_well_formed_block(1, 1, 2, 3, 5, Some(proof));
        let result = block.verify_well_formed();
        assert!(result.is_err(), "proof.round < lppr should fail");
        assert!(result.unwrap_err().to_string().contains("Primary proof round"));
    }

    #[test]
    fn test_verify_well_formed_proof_round_equals_lppr() {
        // proof.round (3) == lppr (3) → should pass (correct cutting block)
        let proof = PrimaryConsensusProof::QC(make_qc(1, 3, 500));
        let block = make_well_formed_block(1, 1, 2, 3, 3, Some(proof));
        assert!(block.verify_well_formed().is_ok());
    }

    #[test]
    fn test_verify_well_formed_reconfiguration_rejected() {
        // Grandparent has reconfiguration → optimistic proposals disallowed
        let epoch = 1;
        let grandparent_info = BlockInfo::new(
            epoch, 1, HashValue::random(), HashValue::random(), 0, 500,
            Some(aptos_types::epoch_state::EpochState::empty()), // has reconfiguration
        );
        let gp_vote_data = VoteData::new(grandparent_info.clone(), grandparent_info.clone());
        let gp_li = LedgerInfo::new(grandparent_info, HashValue::zero());
        let gp_li_sig = LedgerInfoWithSignatures::new(gp_li, AggregateSignature::empty());
        let grandparent_qc = QuorumCert::new(gp_vote_data, gp_li_sig);

        let block = OptProxyBlockData::new(
            vec![],
            Payload::empty(false, true),
            Author::random(),
            epoch,
            3,
            3000,
            make_block_info(epoch, 2, 2000),
            grandparent_qc,
            0,
            None,
        );
        let result = block.verify_well_formed();
        assert!(result.is_err(), "Block after reconfiguration should fail");
        assert!(result.unwrap_err().to_string().contains("reconfiguration"));
    }
}
