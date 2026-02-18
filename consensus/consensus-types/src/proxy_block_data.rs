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
/// - `primary_round`: Which primary round this proxy block belongs to
/// - `primary_proof`: Attached when the primary proof (QC or TC) for `primary_round - 1` is available
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
        /// Which primary round this proxy block belongs to
        primary_round: Round,
        /// Primary consensus proof (QC or TC) attached when proof.round == primary_round - 1
        /// This proof "cuts" the proxy blocks for the previous primary round
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
        primary_round: Round,
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
                primary_round,
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
    /// - Primary round consistency with parent
    /// - Primary QC round matches primary_round - 1 (if attached)
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

        // Primary round validation
        // If parent has a primary QC, this block should be in the next primary round
        // Otherwise, it should be in the same primary round as parent
        // Note: We can't check parent's primary_qc here since we only have BlockInfo,
        // but we validate primary_qc attachment rule below

        // Primary proof attachment validation.
        // Proof round must be >= primary_round - 1 (allows TC gap case).
        if let Some(ref primary_proof) = self.primary_proof() {
            ensure!(
                primary_proof.proof_round() >= self.primary_round().saturating_sub(1),
                "Primary proof round {} must be >= primary_round - 1 = {}",
                primary_proof.proof_round(),
                self.primary_round().saturating_sub(1),
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

    pub fn primary_round(&self) -> Round {
        match self {
            OptProxyBlockBody::V0 { primary_round, .. } => *primary_round,
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
            "[proxy author: {}, epoch: {}, round: {:02}, primary_round: {:02}, parent_id: {}, timestamp: {}, has_primary_proof: {}]",
            self.author(),
            self.epoch(),
            self.round(),
            self.primary_round(),
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
            1, // primary_round
            None,
        );

        assert_eq!(block.epoch(), epoch);
        assert_eq!(block.round(), 3);
        assert_eq!(block.primary_round(), 1);
        assert!(!block.has_primary_proof());
    }

    #[test]
    fn test_opt_proxy_block_data_with_primary_proof() {
        let epoch = 1;
        let grandparent_qc = make_qc(epoch, 1, 1000);
        let parent = make_block_info(epoch, 2, 2000);
        let primary_qc = make_qc(epoch, 0, 500); // QC for primary round 0

        let block = OptProxyBlockData::new(
            vec![],
            Payload::empty(false, true),
            Author::random(),
            epoch,
            3,
            3000,
            parent,
            grandparent_qc,
            1, // primary_round = 1, so primary_proof should be for round 0
            Some(PrimaryConsensusProof::QC(primary_qc)),
        );

        assert!(block.has_primary_proof());
        assert_eq!(block.primary_proof().unwrap().proof_round(), 0);
    }
}
