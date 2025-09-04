// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::Round, quorum_cert::QuorumCert, timeout_2chain::TwoChainTimeoutCertificate,
    wrapped_ledger_info::WrappedLedgerInfo,
};
use anyhow::{ensure, Context};
use velor_types::{block_info::BlockInfo, validator_verifier::ValidatorVerifier};
use fail::fail_point;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

#[derive(Deserialize, Serialize, Clone, Eq, PartialEq)]
/// This struct describes basic synchronization metadata.
pub struct SyncInfo {
    /// Highest quorum certificate known to the peer.
    highest_quorum_cert: QuorumCert,
    /// Highest ordered cert known to the peer.
    highest_ordered_cert: Option<WrappedLedgerInfo>,
    /// Highest commit cert (ordered cert with execution result) known to the peer.
    highest_commit_cert: WrappedLedgerInfo,
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
            "SyncInfo[certified_round: {}, ordered_round: {}, timeout round: {}, committed_round: {},\n hqc: {},\n hoc: {},\n hcc: {}]",
            self.highest_certified_round(),
            self.highest_ordered_round(),
            self.highest_timeout_round(),
            self.highest_commit_round(),
            self.highest_quorum_cert,
            self.highest_ordered_cert.as_ref().map_or_else(|| "None".to_string(), |cert| cert.to_string()),
            self.highest_commit_cert,
        )
    }
}

impl SyncInfo {
    pub fn new_decoupled(
        highest_quorum_cert: QuorumCert,
        highest_ordered_cert: WrappedLedgerInfo,
        highest_commit_cert: WrappedLedgerInfo,
        highest_2chain_timeout_cert: Option<TwoChainTimeoutCertificate>,
    ) -> Self {
        // No need to include HTC if it's lower than HQC
        let highest_2chain_timeout_cert = highest_2chain_timeout_cert
            .filter(|tc| tc.round() > highest_quorum_cert.certified_block().round());

        fail_point!("consensus::ordered_only_cert", |_| {
            Self {
                highest_quorum_cert: highest_quorum_cert.clone(),
                highest_ordered_cert: Some(highest_ordered_cert.clone()),
                highest_commit_cert: highest_ordered_cert.clone(),
                highest_2chain_timeout_cert: highest_2chain_timeout_cert.clone(),
            }
        });

        Self {
            highest_quorum_cert,
            highest_ordered_cert: Some(highest_ordered_cert),
            highest_commit_cert,
            highest_2chain_timeout_cert,
        }
    }

    pub fn new(
        highest_quorum_cert: QuorumCert,
        highest_ordered_cert: WrappedLedgerInfo,
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
    pub fn highest_ordered_cert(&self) -> WrappedLedgerInfo {
        if let Some(cert) = &self.highest_ordered_cert {
            cert.clone()
        } else {
            self.highest_quorum_cert.into_wrapped_ledger_info()
        }
    }

    /// Highest ledger info
    pub fn highest_commit_cert(&self) -> &WrappedLedgerInfo {
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
            epoch == self.highest_ordered_cert().commit_info().epoch(),
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
                >= self.highest_ordered_cert().commit_info().round(),
            "HQC has lower round than HOC"
        );

        ensure!(
            self.highest_ordered_round() >= self.highest_commit_round(),
            format!(
                "HOC {} has lower round than HLI {}",
                self.highest_ordered_cert(),
                self.highest_commit_cert()
            )
        );

        ensure!(
            *self.highest_ordered_cert().commit_info() != BlockInfo::empty(),
            "HOC has no committed block"
        );

        ensure!(
            *self.highest_commit_cert().commit_info() != BlockInfo::empty(),
            "HLI has empty commit info"
        );

        // we don't have execution in unit tests, so this check would fail
        #[cfg(not(any(test, feature = "fuzzing")))]
        {
            ensure!(
                !self.highest_commit_cert().commit_info().is_ordered_only(),
                "HLI {} has ordered only commit info",
                self.highest_commit_cert().commit_info()
            );
        }

        self.highest_quorum_cert
            .verify(validator)
            .and_then(|_| {
                self.highest_ordered_cert
                    .as_ref()
                    .map_or(Ok(()), |cert| cert.verify(validator))
                    .context("Fail to verify ordered certificate")
            })
            .and_then(|_| {
                // we do not verify genesis ledger info
                if self.highest_commit_cert.commit_info().round() > 0 {
                    self.highest_commit_cert
                        .verify(validator)
                        .context("Fail to verify commit certificate")?
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
