// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{Author, Payload, Round},
    payload::OptQuorumStorePayload,
    quorum_cert::QuorumCert,
};
use anyhow::ensure;
use aptos_crypto::HashValue;
use aptos_types::{validator_txn::ValidatorTransaction, validator_verifier::ValidatorVerifier};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum OptBlockBody {
    V0 {
        validator_txns: Vec<ValidatorTransaction>,
        // T of the block (e.g. one or more transaction(s)
        payload: Payload,
        // Author of the block that can be validated by the author's public key and the signature
        author: Author,
        // QC of the grandparent block
        grandparent_qc: QuorumCert,
    },
}

impl OptBlockBody {
    pub fn author(&self) -> &Author {
        match self {
            OptBlockBody::V0 { author, .. } => author,
        }
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        match self {
            OptBlockBody::V0 { validator_txns, .. } => Some(validator_txns),
        }
    }

    pub fn payload(&self) -> &Payload {
        match self {
            OptBlockBody::V0 { payload, .. } => payload,
        }
    }

    pub fn grandparent_qc(&self) -> &QuorumCert {
        match self {
            OptBlockBody::V0 { grandparent_qc, .. } => grandparent_qc,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum ProposalExt {
    V0 {
        validator_txns: Vec<ValidatorTransaction>,
        /// T of the block (e.g. one or more transaction(s)
        payload: Payload,
        /// Author of the block that can be validated by the author's public key and the signature
        author: Author,
        /// Failed authors from the parent's block to this block.
        /// I.e. the list of consecutive proposers from the
        /// immediately preceeding rounds that didn't produce a successful block.
        failed_authors: Vec<(Round, Author)>,
    },
}

impl ProposalExt {
    pub fn author(&self) -> &Author {
        match self {
            ProposalExt::V0 { author, .. } => author,
        }
    }

    pub fn failed_authors(&self) -> &Vec<(Round, Author)> {
        match self {
            ProposalExt::V0 { failed_authors, .. } => failed_authors,
        }
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        match self {
            ProposalExt::V0 { validator_txns, .. } => Some(validator_txns),
        }
    }

    pub fn payload(&self) -> Option<&Payload> {
        match self {
            ProposalExt::V0 { payload, .. } => Some(payload),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MoonBlockMetadata {
    pub earth_round: Round,
    pub earth_qc: Option<QuorumCert>,
}

impl MoonBlockMetadata {
    pub fn verify(&self, moon_verifier: &ValidatorVerifier) -> anyhow::Result<()> {
        if let Some(qc) = &self.earth_qc {
            qc.verify(moon_verifier)?;
            ensure!(
                qc.certified_block().round() == self.earth_round - 1,
                "Invalid earth QC round"
            );
        }
        Ok(())
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum MoonBlock {
    V0 {
        metadata: MoonBlockMetadata,
        payload: OptQuorumStorePayload,
    },
}

impl MoonBlock {
    pub fn metadata(&self) -> &MoonBlockMetadata {
        match self {
            MoonBlock::V0 { metadata, .. } => metadata,
        }
    }

    pub fn payload(&self) -> &OptQuorumStorePayload {
        match self {
            MoonBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn payload_mut(&mut self) -> &mut OptQuorumStorePayload {
        match self {
            MoonBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn take_payload(self) -> OptQuorumStorePayload {
        match self {
            MoonBlock::V0 { payload, .. } => payload,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MoonBlockFullMetadata {
    pub moon_block_metadata: MoonBlockMetadata,
    pub moon_id: HashValue,
    pub moon_author: Author,
    pub moon_round: Round,
    pub moon_timestamp_usecs: u64,
    pub moon_qc: QuorumCert,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct EarthBlockMetadata {
    pub moon_block_full_metadata: Vec<MoonBlockFullMetadata>,
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum EarthBlock {
    V0 {
        metadata: EarthBlockMetadata,
        payload: OptQuorumStorePayload,
    },
}

impl EarthBlock {
    pub fn metadata(&self) -> &EarthBlockMetadata {
        match self {
            EarthBlock::V0 { metadata, .. } => metadata,
        }
    }

    pub fn payload(&self) -> &OptQuorumStorePayload {
        match self {
            EarthBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn payload_mut(&mut self) -> &mut OptQuorumStorePayload {
        match self {
            EarthBlock::V0 { payload, .. } => payload,
        }
    }

    pub fn take_payload(self) -> OptQuorumStorePayload {
        match self {
            EarthBlock::V0 { payload, .. } => payload,
        }
    }
}
