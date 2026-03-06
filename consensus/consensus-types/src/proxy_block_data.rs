// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Proxy block data types for proxy primary consensus.
//!
//! Proxy blocks are produced by a subset of validators (proxies) running fast local consensus.
//! They are aggregated into primary blocks by the full validator set.

use crate::{
    common::{Author, Payload, Round},
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
/// Pure BFT proxy blocks — no primary consensus linkage fields.
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

        Ok(())
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
            "[proxy author: {}, epoch: {}, round: {:02}, parent_id: {}, timestamp: {}]",
            self.author(),
            self.epoch(),
            self.round(),
            self.parent_id(),
            self.timestamp_usecs(),
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
        );

        assert_eq!(block.epoch(), epoch);
        assert_eq!(block.round(), 3);
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
        )
    }

    #[test]
    fn test_verify_well_formed_happy_path() {
        // grandparent=1, parent=2, block=3
        let block = make_well_formed_block(1, 1, 2, 3);
        assert!(block.verify_well_formed().is_ok());
    }

    #[test]
    fn test_verify_well_formed_grandparent_parent_gap() {
        // grandparent=1, parent=3 → parent.round != grandparent.round + 1
        let block = make_well_formed_block(1, 1, 3, 4);
        let result = block.verify_well_formed();
        assert!(result.is_err(), "Gap between grandparent and parent round should fail");
        assert!(result.unwrap_err().to_string().contains("one more than grandparent"));
    }

    #[test]
    fn test_verify_well_formed_parent_block_gap() {
        // grandparent=1, parent=2, block=4 → block.round != parent.round + 1
        let block = make_well_formed_block(1, 1, 2, 4);
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
        );
        let result = block.verify_well_formed();
        assert!(result.is_err(), "Non-increasing timestamp should fail");
        assert!(result.unwrap_err().to_string().contains("timestamp"));
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
        );
        let result = block.verify_well_formed();
        assert!(result.is_err(), "Block after reconfiguration should fail");
        assert!(result.unwrap_err().to_string().contains("reconfiguration"));
    }
}
