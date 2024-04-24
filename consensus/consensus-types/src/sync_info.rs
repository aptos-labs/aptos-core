// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{common::Round, quorum_cert::QuorumCert, timeout_2chain::TwoChainTimeoutCertificate};
use anyhow::{ensure, Context};
use aptos_types::{
    block_info::BlockInfo, ledger_info::LedgerInfoWithSignatures,
    validator_verifier::ValidatorVerifier,
};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

/// This struct describes basic synchronization metadata. This struct is versioned to allow
/// upgrades.
#[derive(Deserialize, Serialize, Clone, Eq, PartialEq)]
pub enum VersionedSyncInfo {
    V1(SyncInfo),
    V2(SyncInfoV2),
}

// this is required by structured log
impl Debug for VersionedSyncInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for VersionedSyncInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            VersionedSyncInfo::V1(sync_info) => write!(f, "V1({})", sync_info),
            VersionedSyncInfo::V2(sync_info) => write!(f, "V2({})", sync_info),
        }
    }
}

impl VersionedSyncInfo {
    pub fn new_v1(sync_info: SyncInfo) -> Self {
        VersionedSyncInfo::V1(sync_info)
    }

    pub fn new_v2(sync_info: SyncInfoV2) -> Self {
        VersionedSyncInfo::V2(sync_info)
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        match self {
            VersionedSyncInfo::V1(sync_info) => sync_info.verify(validator),
            VersionedSyncInfo::V2(sync_info) => sync_info.verify(validator),
        }
    }

    pub fn highest_round(&self) -> Round {
        match self {
            VersionedSyncInfo::V1(sync_info) => sync_info.highest_round(),
            VersionedSyncInfo::V2(sync_info) => sync_info.highest_round(),
        }
    }

    pub fn highest_certified_round(&self) -> Round {
        match self {
            VersionedSyncInfo::V1(sync_info) => sync_info.highest_certified_round(),
            VersionedSyncInfo::V2(sync_info) => sync_info.highest_certified_round(),
        }
    }

    pub fn highest_timeout_round(&self) -> Round {
        match self {
            VersionedSyncInfo::V1(sync_info) => sync_info.highest_timeout_round(),
            VersionedSyncInfo::V2(sync_info) => sync_info.highest_timeout_round(),
        }
    }

    pub fn highest_ordered_round(&self) -> Round {
        match self {
            VersionedSyncInfo::V1(sync_info) => sync_info.highest_ordered_round(),
            VersionedSyncInfo::V2(sync_info) => sync_info.highest_ordered_round(),
        }
    }

    pub fn highest_commit_round(&self) -> Round {
        match self {
            VersionedSyncInfo::V1(sync_info) => sync_info.highest_commit_round(),
            VersionedSyncInfo::V2(sync_info) => sync_info.highest_commit_round(),
        }
    }

    pub fn highest_quorum_cert(&self) -> &QuorumCert {
        match self {
            VersionedSyncInfo::V1(sync_info) => sync_info.highest_quorum_cert(),
            VersionedSyncInfo::V2(sync_info) => sync_info.highest_quorum_cert(),
        }
    }

    pub fn epoch(&self) -> u64 {
        match self {
            VersionedSyncInfo::V1(sync_info) => sync_info.epoch(),
            VersionedSyncInfo::V2(sync_info) => sync_info.epoch(),
        }
    }

    pub fn highest_2chain_timeout_cert(&self) -> Option<&TwoChainTimeoutCertificate> {
        match self {
            VersionedSyncInfo::V1(sync_info) => sync_info.highest_2chain_timeout_cert(),
            VersionedSyncInfo::V2(sync_info) => sync_info.highest_2chain_timeout_cert(),
        }
    }

    pub fn has_newer_certificates(&self, other: &VersionedSyncInfo) -> bool {
        self.highest_certified_round() > other.highest_certified_round()
            || self.highest_timeout_round() > other.highest_timeout_round()
            || self.highest_ordered_round() > other.highest_ordered_round()
            || self.highest_commit_round() > other.highest_commit_round()
    }
}

#[derive(Deserialize, Serialize, Clone, Eq, PartialEq)]
/// This struct describes basic synchronization metadata.
pub struct SyncInfo {
    /// Highest quorum certificate known to the peer.
    highest_quorum_cert: QuorumCert,
    /// Highest ordered cert known to the peer.
    highest_ordered_cert: Option<QuorumCert>,
    /// Highest commit cert (ordered cert with execution result) known to the peer.
    highest_commit_cert: QuorumCert,
    /// Optional highest timeout certificate if available.
    highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
}

// this is required by structured log
impl Debug for SyncInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for SyncInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "SyncInfo[certified_round: {}, ordered_round: {}, timeout round: {}, commit_info: {}]",
            self.highest_certified_round(),
            self.highest_ordered_round(),
            self.highest_timeout_round(),
            self.highest_commit_cert().commit_info(),
        )
    }
}

impl SyncInfo {
    pub fn new_decoupled(
        highest_quorum_cert: QuorumCert,
        highest_ordered_cert: QuorumCert,
        highest_commit_cert: QuorumCert,
        highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
    ) -> Self {
        // No need to include HTC if it's lower than HQC
        let highest_2chain_timeout_cert = highest_2chain_timeout_cert
            .filter(|tc| tc.round() > highest_quorum_cert.certified_block().round());

        let highest_ordered_cert =
            Some(highest_ordered_cert).filter(|hoc| hoc != &highest_quorum_cert);

        Self {
            highest_quorum_cert,
            highest_ordered_cert,
            highest_commit_cert,
            highest_2chain_timeout_cert,
        }
    }

    pub fn new(
        highest_quorum_cert: QuorumCert,
        highest_ordered_cert: QuorumCert,
        highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
    ) -> Self {
        let highest_commit_cert = highest_ordered_cert.clone();
        Self::new_decoupled(
            highest_quorum_cert,
            highest_ordered_cert,
            highest_commit_cert,
            highest_2chain_timeout_cert,
        )
    }

    /// Highest quorum certificate
    pub fn highest_quorum_cert(&self) -> &QuorumCert {
        &self.highest_quorum_cert
    }

    /// Highest ordered certificate
    pub fn highest_ordered_cert(&self) -> &QuorumCert {
        self.highest_ordered_cert
            .as_ref()
            .unwrap_or(&self.highest_quorum_cert)
    }

    /// Highest ledger info
    pub fn highest_commit_cert(&self) -> &QuorumCert {
        &self.highest_commit_cert
    }

    /// Highest 2-chain timeout certificate
    pub fn highest_2chain_timeout_cert(&self) -> Option<&TwoChainTimeoutCertificate> {
        self.highest_2chain_timeout_cert.as_ref()
    }

    pub fn highest_certified_round(&self) -> Round {
        self.highest_quorum_cert.certified_block().round()
    }

    pub fn highest_timeout_round(&self) -> Round {
        self.highest_2chain_timeout_cert()
            .map_or(0, |tc| tc.round())
    }

    pub fn highest_ordered_round(&self) -> Round {
        self.highest_ordered_cert().commit_info().round()
    }

    pub fn highest_commit_round(&self) -> Round {
        self.highest_commit_cert().commit_info().round()
    }

    /// The highest round the SyncInfo carries.
    pub fn highest_round(&self) -> Round {
        std::cmp::max(self.highest_certified_round(), self.highest_timeout_round())
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        let epoch = self.highest_quorum_cert.certified_block().epoch();
        ensure!(
            epoch == self.highest_ordered_cert().certified_block().epoch(),
            "Multi epoch in SyncInfo - HOC and HQC"
        );
        ensure!(
            epoch == self.highest_commit_cert().commit_info().epoch(),
            "Multi epoch in SyncInfo - HOC and HCC"
        );
        if let Some(tc) = &self.highest_2chain_timeout_cert {
            ensure!(epoch == tc.epoch(), "Multi epoch in SyncInfo - TC and HQC");
        }

        ensure!(
            self.highest_quorum_cert.certified_block().round()
                >= self.highest_ordered_cert().certified_block().round(),
            "HQC has lower round than HOC"
        );

        ensure!(
            self.highest_ordered_cert().certified_block().round() >= self.highest_commit_round(),
            "HOC has lower round than HLI"
        );

        ensure!(
            *self.highest_ordered_cert().commit_info() != BlockInfo::empty(),
            "HOC has no committed block"
        );

        ensure!(
            *self.highest_commit_cert().commit_info() != BlockInfo::empty(),
            "HLI has empty commit info"
        );

        self.highest_quorum_cert
            .verify(validator)
            .and_then(|_| {
                self.highest_ordered_cert
                    .as_ref()
                    .map_or(Ok(()), |cert| cert.verify(validator))
            })
            .and_then(|_| {
                // we do not verify genesis ledger info
                if self.highest_commit_cert.commit_info().round() > 0 {
                    self.highest_commit_cert.verify(validator)?;
                }
                Ok(())
            })
            .and_then(|_| {
                if let Some(tc) = &self.highest_2chain_timeout_cert {
                    tc.verify(validator)?;
                }
                Ok(())
            })
            .context("Fail to verify SyncInfo")?;
        Ok(())
    }

    pub fn epoch(&self) -> u64 {
        self.highest_quorum_cert.certified_block().epoch()
    }

    pub fn has_newer_certificates(&self, other: &SyncInfo) -> bool {
        self.highest_certified_round() > other.highest_certified_round()
            || self.highest_timeout_round() > other.highest_timeout_round()
            || self.highest_ordered_round() > other.highest_ordered_round()
            || self.highest_commit_round() > other.highest_commit_round()
    }
}

#[derive(Deserialize, Serialize, Clone, Eq, PartialEq)]
/// This struct describes basic synchronization metadata.
pub struct SyncInfoV2 {
    /// Highest quorum certificate known to the peer.
    highest_quorum_cert: QuorumCert,
    /// Highest ordered decision known to the peer.
    highest_ordered_decision: Option<LedgerInfoWithSignatures>,
    /// Highest commit decision (ordered decision with execution result) known to the peer.
    highest_commit_decision: LedgerInfoWithSignatures,
    /// Optional highest timeout certificate if available.
    highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
}

// this is required by structured log
impl Debug for SyncInfoV2 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl Display for SyncInfoV2 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "SyncInfo[highest_quorum_cert: {:?}, highest_ordered_decision: {:?}, highest_commit_decision: {:?}, highest_2chain_timeout_cert: {:?}, certified_round: {}, ordered_round: {}, timeout round: {}, commit_info: {}]",
            self.highest_quorum_cert,
            self.highest_ordered_decision,
            self.highest_commit_decision.commit_info(),
            self.highest_2chain_timeout_cert,
            self.highest_certified_round(),
            self.highest_ordered_round(),
            self.highest_timeout_round(),
            self.highest_commit_decision().commit_info(),
        )
    }
}

impl SyncInfoV2 {
    pub fn new_decoupled(
        highest_quorum_cert: QuorumCert,
        highest_ordered_decision: LedgerInfoWithSignatures,
        highest_commit_decision: LedgerInfoWithSignatures,
        highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
    ) -> Self {
        // No need to include HTC if it's lower than HQC
        let highest_2chain_timeout_cert = highest_2chain_timeout_cert
            .filter(|tc| tc.round() > highest_quorum_cert.certified_block().round());

        // TODO: Is this an okay check to do? Here we are also check if consensus_data_hash is matching.
        let highest_ordered_decision =
            Some(highest_ordered_decision).filter(|hoc| hoc != highest_quorum_cert.ledger_info());

        Self {
            highest_quorum_cert,
            highest_ordered_decision,
            highest_commit_decision,
            highest_2chain_timeout_cert,
        }
    }

    pub fn new(
        highest_quorum_cert: QuorumCert,
        highest_ordered_decision: LedgerInfoWithSignatures,
        highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
    ) -> Self {
        let highest_commit_decision = highest_ordered_decision.clone();
        Self::new_decoupled(
            highest_quorum_cert,
            highest_ordered_decision,
            highest_commit_decision,
            highest_2chain_timeout_cert,
        )
    }

    /// Highest quorum certificate
    pub fn highest_quorum_cert(&self) -> &QuorumCert {
        &self.highest_quorum_cert
    }

    /// Highest ordered certificate
    pub fn highest_ordered_decision(&self) -> &LedgerInfoWithSignatures {
        self.highest_ordered_decision
            .as_ref()
            .unwrap_or(self.highest_quorum_cert.ledger_info())
    }

    /// Highest ledger info
    pub fn highest_commit_decision(&self) -> &LedgerInfoWithSignatures {
        &self.highest_commit_decision
    }

    /// Highest 2-chain timeout certificate
    pub fn highest_2chain_timeout_cert(&self) -> Option<&TwoChainTimeoutCertificate> {
        self.highest_2chain_timeout_cert.as_ref()
    }

    pub fn highest_certified_round(&self) -> Round {
        self.highest_quorum_cert.certified_block().round()
    }

    pub fn highest_timeout_round(&self) -> Round {
        self.highest_2chain_timeout_cert()
            .map_or(0, |tc| tc.round())
    }

    pub fn highest_ordered_round(&self) -> Round {
        self.highest_ordered_decision().commit_info().round()
    }

    pub fn highest_commit_round(&self) -> Round {
        self.highest_commit_decision().commit_info().round()
    }

    /// The highest round the SyncInfo carries.
    pub fn highest_round(&self) -> Round {
        std::cmp::max(self.highest_certified_round(), self.highest_timeout_round())
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        let epoch = self.highest_quorum_cert.certified_block().epoch();
        // TODO: Is it okay to change highest_ordered_cert -> highest_ordered_decision here?
        // Does it break when epoch changes?
        ensure!(
            epoch == self.highest_ordered_decision().commit_info().epoch(),
            "Multi epoch in SyncInfo - HOC and HQC"
        );
        ensure!(
            epoch == self.highest_commit_decision().commit_info().epoch(),
            "Multi epoch in SyncInfo - HOC and HCC"
        );
        if let Some(tc) = &self.highest_2chain_timeout_cert {
            ensure!(epoch == tc.epoch(), "Multi epoch in SyncInfo - TC and HQC");
        }

        ensure!(
            self.highest_quorum_cert.certified_block().round()
                >= self.highest_ordered_decision().commit_info().round(),
            "HQC has lower round than HOC"
        );

        ensure!(
            self.highest_ordered_decision().commit_info().round() >= self.highest_commit_round(),
            "HOC has lower round than HLI"
        );

        ensure!(
            *self.highest_ordered_decision().commit_info() != BlockInfo::empty(),
            "HOC has no committed block"
        );

        ensure!(
            *self.highest_commit_decision().commit_info() != BlockInfo::empty(),
            "HLI has empty commit info"
        );

        self.highest_quorum_cert
            .verify(validator)
            .and_then(|_| {
                if let Some(highest_ordered_decision) = &self.highest_ordered_decision {
                    // TODO: Earlier, quroum_cert.verify() compares if the certified_block.round() is 0.
                    // Here, we are comparing commit_info.round() > 0. Is this okay?
                    if highest_ordered_decision.commit_info().round() > 0 {
                        highest_ordered_decision.verify_signatures(validator)?;
                    }
                }
                Ok(())
            })
            .and_then(|_| {
                // we do not verify genesis ledger info
                if self.highest_commit_decision.commit_info().round() > 0 {
                    self.highest_commit_decision.verify_signatures(validator)?;
                }
                Ok(())
            })
            .and_then(|_| {
                if let Some(tc) = &self.highest_2chain_timeout_cert {
                    tc.verify(validator)?;
                }
                Ok(())
            })
            .context("Fail to verify SyncInfo")?;
        Ok(())
    }

    pub fn epoch(&self) -> u64 {
        self.highest_quorum_cert.certified_block().epoch()
    }

    pub fn has_newer_certificates(&self, other: &SyncInfo) -> bool {
        self.highest_certified_round() > other.highest_certified_round()
            || self.highest_timeout_round() > other.highest_timeout_round()
            || self.highest_ordered_round() > other.highest_ordered_round()
            || self.highest_commit_round() > other.highest_commit_round()
    }
}
