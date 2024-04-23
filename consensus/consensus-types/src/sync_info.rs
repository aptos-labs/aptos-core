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
    /// Highest ordered decision known to the peer.
    highest_ordered_decision: Option<LedgerInfoWithSignatures>,
    /// Highest commit decision known to the peer.
    highest_commit_decision: Option<LedgerInfoWithSignatures>,
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
            "SyncInfo[certified_round: {}, ordered_round: {}, timeout round: {}, commit_info: {}, ordered_decision: {:?}, commit_decision: {:?}]",
            self.highest_certified_round(),
            self.highest_ordered_round(),
            self.highest_timeout_round(),
            self.highest_commit_cert().commit_info(),
            self.highest_ordered_decision(),
            self.highest_commit_decision(),
        )
    }
}

impl SyncInfo {
    pub fn new_decoupled(
        highest_quorum_cert: QuorumCert,
        highest_ordered_cert: QuorumCert,
        highest_commit_cert: QuorumCert,
        highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
        highest_ordered_decision: Option<LedgerInfoWithSignatures>,
        highest_commit_decision: Option<LedgerInfoWithSignatures>,
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
            highest_ordered_decision,
            highest_commit_decision,
        }
    }

    pub fn new(
        highest_quorum_cert: QuorumCert,
        highest_ordered_cert: QuorumCert,
        highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
        highest_ordered_decision: Option<LedgerInfoWithSignatures>,
    ) -> Self {
        let highest_commit_cert = highest_ordered_cert.clone();
        let highest_commit_decision = highest_ordered_decision.clone();
        Self::new_decoupled(
            highest_quorum_cert,
            highest_ordered_cert,
            highest_commit_cert,
            highest_2chain_timeout_cert,
            highest_ordered_decision,
            highest_commit_decision,
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

    pub fn highest_commit_decision(&self) -> &Option<LedgerInfoWithSignatures> {
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
        self.highest_ordered_decision.as_ref().map_or(
            self.highest_ordered_cert().commit_info().round(),
            |decision| decision.commit_info().round(),
        )
    }

    pub fn highest_ordered_decision(&self) -> &Option<LedgerInfoWithSignatures> {
        &self.highest_ordered_decision
    }

    pub fn highest_commit_round(&self) -> Round {
        self.highest_commit_decision.as_ref().map_or(
            self.highest_commit_cert().commit_info().round(),
            |decision| decision.commit_info().round(),
        )
    }

    fn highest_commit_epoch(&self) -> u64 {
        self.highest_commit_decision.as_ref().map_or(
            self.highest_commit_cert().commit_info().epoch(),
            |decision| decision.commit_info().epoch(),
        )
    }

    fn highest_ordered_epoch(&self) -> u64 {
        self.highest_ordered_decision.as_ref().map_or(
            self.highest_ordered_cert().commit_info().epoch(),
            |decision| decision.commit_info().epoch(),
        )
    }

    /// The highest round the SyncInfo carries.
    pub fn highest_round(&self) -> Round {
        std::cmp::max(self.highest_certified_round(), self.highest_timeout_round())
    }

    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        let epoch = self.highest_quorum_cert.certified_block().epoch();
        // TODO: Earlier, we compred highest_ordered_cert.certified_block().epoch() with epoch.
        // Now, we are comparing highest_ordered_cert.commit_info().epoch() with epoch. Is this okay?
        ensure!(
            epoch == self.highest_ordered_epoch(),
            "Multi epoch in SyncInfo - HOC and HQC"
        );
        ensure!(
            epoch == self.highest_commit_epoch(),
            "Multi epoch in SyncInfo - HOC and HCC"
        );
        if let Some(tc) = &self.highest_2chain_timeout_cert {
            ensure!(epoch == tc.epoch(), "Multi epoch in SyncInfo - TC and HQC");
        }

        ensure!(
            self.highest_quorum_cert.certified_block().round() >= self.highest_ordered_round(),
            "HQC has lower round than HOC"
        );

        ensure!(
            self.highest_ordered_round() >= self.highest_commit_round(),
            "HOC has lower round than HLI"
        );

        ensure!(
            *self
                .highest_ordered_decision
                .as_ref()
                .map_or(self.highest_ordered_cert().commit_info(), |decision| {
                    decision.commit_info()
                })
                != BlockInfo::empty(),
            "HOC has no committed block"
        );

        ensure!(
            *self
                .highest_commit_decision
                .as_ref()
                .map_or(self.highest_commit_cert().commit_info(), |decision| {
                    decision.commit_info()
                })
                != BlockInfo::empty(),
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
                } else {
                    self.highest_ordered_cert
                        .as_ref()
                        .map_or(Ok(()), |cert| cert.verify(validator))?
                }
                Ok(())
            })
            .and_then(|_| {
                // we do not verify genesis ledger info
                if self.highest_commit_round() > 0 {
                    if let Some(highest_commit_decision) = &self.highest_commit_decision {
                        highest_commit_decision.verify_signatures(validator)?;
                    } else {
                        self.highest_commit_cert.verify(validator)?
                    }
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
