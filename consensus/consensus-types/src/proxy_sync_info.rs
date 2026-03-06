// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! ProxySyncInfo tracks synchronization state for both proxy and primary consensus.
//!
//! Unlike SyncInfo, ProxySyncInfo:
//! - Tracks both proxy and primary consensus state
//! - Carries the proxy's commit cert separately from ordered cert, since the proxy
//!   never executes blocks and its commit cert stays at genesis (non-ordered-only)

use crate::{
    common::Round, quorum_cert::QuorumCert, timeout_2chain::TwoChainTimeoutCertificate,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use anyhow::{ensure, Context};
use aptos_types::validator_verifier::ValidatorVerifier;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Deserialize, Serialize, Clone, Eq, PartialEq)]
/// Synchronization metadata for proxy consensus, tracking both proxy and primary state.
pub struct ProxySyncInfo {
    // Proxy consensus state
    /// Highest proxy QC known to the peer.
    highest_proxy_qc: QuorumCert,
    /// Highest proxy ordered cert (uses LedgerInfo with default execution state).
    /// None if no proxy blocks have been ordered yet.
    highest_proxy_ordered_cert: Option<WrappedLedgerInfo>,
    /// Highest proxy commit cert from the proxy's BlockStore. The proxy never executes
    /// blocks, so this stays at the genesis value (with real executed state). This must
    /// be carried separately from the ordered cert because ordered certs have
    /// ACCUMULATOR_PLACEHOLDER_HASH / version=0 which fails SyncInfo::verify().
    highest_proxy_commit_cert: WrappedLedgerInfo,
    /// Highest proxy timeout cert if available.
    highest_proxy_timeout_cert: Option<TwoChainTimeoutCertificate>,

    // Primary consensus state (received from primary RoundManager via internal channel)
    /// Highest primary QC known to the peer.
    highest_primary_qc: QuorumCert,
    /// Highest primary TC if available.
    highest_primary_tc: Option<TwoChainTimeoutCertificate>,
}

impl Debug for ProxySyncInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for ProxySyncInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "ProxySyncInfo[proxy_qc_round: {}, proxy_ordered_round: {}, proxy_tc_round: {}, primary_qc_round: {}, primary_tc_round: {}]",
            self.highest_proxy_certified_round(),
            self.highest_proxy_ordered_round(),
            self.highest_proxy_timeout_round(),
            self.highest_primary_certified_round(),
            self.highest_primary_timeout_round(),
        )
    }
}

impl ProxySyncInfo {
    pub fn new(
        highest_proxy_qc: QuorumCert,
        highest_proxy_ordered_cert: Option<WrappedLedgerInfo>,
        highest_proxy_commit_cert: WrappedLedgerInfo,
        highest_proxy_timeout_cert: Option<TwoChainTimeoutCertificate>,
        highest_primary_qc: QuorumCert,
        highest_primary_tc: Option<TwoChainTimeoutCertificate>,
    ) -> Self {
        // Filter out TC if it's lower than QC
        let highest_proxy_timeout_cert = highest_proxy_timeout_cert
            .filter(|tc| tc.round() > highest_proxy_qc.certified_block().round());
        let highest_primary_tc = highest_primary_tc
            .filter(|tc| tc.round() > highest_primary_qc.certified_block().round());

        Self {
            highest_proxy_qc,
            highest_proxy_ordered_cert,
            highest_proxy_commit_cert,
            highest_proxy_timeout_cert,
            highest_primary_qc,
            highest_primary_tc,
        }
    }

    // ========== Proxy Consensus Accessors ==========

    /// Highest proxy quorum certificate
    pub fn highest_proxy_qc(&self) -> &QuorumCert {
        &self.highest_proxy_qc
    }

    /// Highest proxy ordered certificate
    pub fn highest_proxy_ordered_cert(&self) -> Option<&WrappedLedgerInfo> {
        self.highest_proxy_ordered_cert.as_ref()
    }

    /// Highest proxy commit certificate (stays at genesis since proxy doesn't execute)
    pub fn highest_proxy_commit_cert(&self) -> &WrappedLedgerInfo {
        &self.highest_proxy_commit_cert
    }

    /// Highest proxy timeout certificate
    pub fn highest_proxy_timeout_cert(&self) -> Option<&TwoChainTimeoutCertificate> {
        self.highest_proxy_timeout_cert.as_ref()
    }

    pub fn highest_proxy_certified_round(&self) -> Round {
        self.highest_proxy_qc.certified_block().round()
    }

    pub fn highest_proxy_timeout_round(&self) -> Round {
        self.highest_proxy_timeout_cert
            .as_ref()
            .map_or(0, |tc| tc.round())
    }

    pub fn highest_proxy_ordered_round(&self) -> Round {
        self.highest_proxy_ordered_cert
            .as_ref()
            .map_or(0, |cert| cert.commit_info().round())
    }

    /// The highest proxy round (max of proxy QC and proxy TC).
    pub fn highest_proxy_round(&self) -> Round {
        std::cmp::max(
            self.highest_proxy_certified_round(),
            self.highest_proxy_timeout_round(),
        )
    }

    // ========== Primary Consensus Accessors ==========

    /// Highest primary quorum certificate (received from primary consensus)
    pub fn highest_primary_qc(&self) -> &QuorumCert {
        &self.highest_primary_qc
    }

    /// Highest primary timeout certificate
    pub fn highest_primary_tc(&self) -> Option<&TwoChainTimeoutCertificate> {
        self.highest_primary_tc.as_ref()
    }

    pub fn highest_primary_certified_round(&self) -> Round {
        self.highest_primary_qc.certified_block().round()
    }

    pub fn highest_primary_timeout_round(&self) -> Round {
        self.highest_primary_tc
            .as_ref()
            .map_or(0, |tc| tc.round())
    }

    /// The highest primary round (max of primary QC and primary TC).
    pub fn highest_primary_round(&self) -> Round {
        std::cmp::max(
            self.highest_primary_certified_round(),
            self.highest_primary_timeout_round(),
        )
    }

    /// Current primary round: max(QC_primary.round, TC_primary.round) + 1
    pub fn current_primary_round(&self) -> Round {
        self.highest_primary_round() + 1
    }

    // ========== Verification ==========

    /// Verify the sync info signatures using the proxy validator verifier.
    pub fn verify(&self, proxy_verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        let epoch = self.highest_proxy_qc.certified_block().epoch();

        // Verify proxy certificates are in the same epoch
        if let Some(ordered_cert) = &self.highest_proxy_ordered_cert {
            ensure!(
                epoch == ordered_cert.commit_info().epoch(),
                "Multi epoch in ProxySyncInfo - proxy ordered cert and proxy QC"
            );
        }
        if let Some(tc) = &self.highest_proxy_timeout_cert {
            ensure!(
                epoch == tc.epoch(),
                "Multi epoch in ProxySyncInfo - proxy TC and proxy QC"
            );
        }

        // Note: Primary QC/TC may be from a different validator set verification context,
        // but should be in the same epoch for consistency
        ensure!(
            epoch == self.highest_primary_qc.certified_block().epoch(),
            "Multi epoch in ProxySyncInfo - primary QC and proxy QC"
        );
        if let Some(tc) = &self.highest_primary_tc {
            ensure!(
                epoch == tc.epoch(),
                "Multi epoch in ProxySyncInfo - primary TC and proxy QC"
            );
        }

        // Verify round ordering
        ensure!(
            self.highest_proxy_certified_round() >= self.highest_proxy_ordered_round(),
            "Proxy QC has lower round than proxy ordered cert"
        );

        // Verify proxy certificates
        self.highest_proxy_qc
            .verify(proxy_verifier)
            .context("Failed to verify proxy QC")?;

        if let Some(ordered_cert) = &self.highest_proxy_ordered_cert {
            ordered_cert
                .verify(proxy_verifier)
                .context("Failed to verify proxy ordered cert")?;
        }

        if let Some(tc) = &self.highest_proxy_timeout_cert {
            tc.verify(proxy_verifier)
                .context("Failed to verify proxy TC")?;
        }

        // Note: Primary QC/TC verification is done by primary consensus,
        // we trust them here as they come from the primary RoundManager

        Ok(())
    }

    pub fn epoch(&self) -> u64 {
        self.highest_proxy_qc.certified_block().epoch()
    }

    /// Check if this sync info has newer certificates than another.
    pub fn has_newer_certificates(&self, other: &ProxySyncInfo) -> bool {
        self.highest_proxy_certified_round() > other.highest_proxy_certified_round()
            || self.highest_proxy_timeout_round() > other.highest_proxy_timeout_round()
            || self.highest_proxy_ordered_round() > other.highest_proxy_ordered_round()
            || self.highest_primary_certified_round() > other.highest_primary_certified_round()
            || self.highest_primary_timeout_round() > other.highest_primary_timeout_round()
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

    fn make_qc(epoch: u64, round: Round) -> QuorumCert {
        let block_info = BlockInfo::new(
            epoch,
            round,
            HashValue::random(),
            HashValue::random(),
            0,
            round * 1000,
            None,
        );
        let vote_data = VoteData::new(block_info.clone(), block_info.clone());
        let ledger_info = LedgerInfo::new(block_info, HashValue::zero());
        let li_sig = LedgerInfoWithSignatures::new(ledger_info, AggregateSignature::empty());
        QuorumCert::new(vote_data, li_sig)
    }

    #[test]
    fn test_proxy_sync_info_creation() {
        let proxy_qc = make_qc(1, 5);
        let primary_qc = make_qc(1, 2);
        let commit_cert = proxy_qc.clone().into_wrapped_ledger_info();

        let sync_info = ProxySyncInfo::new(proxy_qc, None, commit_cert, None, primary_qc, None);

        assert_eq!(sync_info.highest_proxy_certified_round(), 5);
        assert_eq!(sync_info.highest_primary_certified_round(), 2);
        assert_eq!(sync_info.current_primary_round(), 3);
        assert_eq!(sync_info.epoch(), 1);
    }

    #[test]
    fn test_proxy_sync_info_highest_round() {
        let proxy_qc = make_qc(1, 5);
        let primary_qc = make_qc(1, 2);
        let commit_cert = proxy_qc.clone().into_wrapped_ledger_info();

        let sync_info = ProxySyncInfo::new(proxy_qc, None, commit_cert, None, primary_qc, None);

        assert_eq!(sync_info.highest_proxy_round(), 5);
        assert_eq!(sync_info.highest_primary_round(), 2);
    }
}
