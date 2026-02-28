// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Unified proof type for primary consensus round completion.
//!
//! Both QC (quorum cert) and TC (timeout cert) prove that a primary round has ended:
//! - QC: the round succeeded and a block was certified
//! - TC: the round timed out and 2f+1 validators moved on
//!
//! For proxy cutting-point semantics, both are treated identically: they update
//! `last_primary_proof_round` and create a cutting point in the proxy block stream.

use crate::{quorum_cert::QuorumCert, timeout_2chain::TwoChainTimeoutCertificate};
use aptos_types::{block_info::Round, validator_verifier::ValidatorVerifier};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// Proof that a primary consensus round has completed.
///
/// Used as the cutting-point proof in proxy-primary consensus. The proxy attaches
/// this proof to the last proxy block in a primary round, and the primary proposer
/// includes it in the next proposal to prove the previous round ended.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum PrimaryConsensusProof {
    /// A quorum certificate proving the round's block was certified.
    QC(QuorumCert),
    /// A timeout certificate proving 2f+1 validators timed out the round.
    TC(TwoChainTimeoutCertificate),
}

impl PrimaryConsensusProof {
    /// The round this proof certifies as complete.
    ///
    /// - QC: the round of the certified block (`qc.certified_block().round()`)
    /// - TC: the timed-out round (`tc.round()`)
    pub fn proof_round(&self) -> Round {
        match self {
            PrimaryConsensusProof::QC(qc) => qc.certified_block().round(),
            PrimaryConsensusProof::TC(tc) => tc.round(),
        }
    }

    /// The epoch of this proof.
    pub fn epoch(&self) -> u64 {
        match self {
            PrimaryConsensusProof::QC(qc) => {
                qc.certified_block().epoch()
            },
            PrimaryConsensusProof::TC(tc) => tc.epoch(),
        }
    }

    /// Verify the cryptographic validity of this proof.
    pub fn verify(&self, validator: &ValidatorVerifier) -> anyhow::Result<()> {
        match self {
            PrimaryConsensusProof::QC(qc) => qc.verify(validator),
            PrimaryConsensusProof::TC(tc) => tc.verify(validator),
        }
    }

    /// Returns true if this is a QC proof.
    pub fn is_qc(&self) -> bool {
        matches!(self, PrimaryConsensusProof::QC(_))
    }

    /// Returns true if this is a TC proof.
    pub fn is_tc(&self) -> bool {
        matches!(self, PrimaryConsensusProof::TC(_))
    }
}

impl Display for PrimaryConsensusProof {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            PrimaryConsensusProof::QC(qc) => {
                write!(
                    f,
                    "PrimaryProof::QC(round={}, epoch={})",
                    qc.certified_block().round(),
                    qc.certified_block().epoch()
                )
            },
            PrimaryConsensusProof::TC(tc) => {
                write!(
                    f,
                    "PrimaryProof::TC(round={}, epoch={})",
                    tc.round(),
                    tc.epoch()
                )
            },
        }
    }
}
