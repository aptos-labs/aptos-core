// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::{Author, Payload, Round},
    primary_consensus_proof::PrimaryConsensusProof,
    quorum_cert::QuorumCert,
};
use aptos_types::validator_txn::ValidatorTransaction;
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
    /// Proxy optimistic block — extends V0 with primary consensus linkage fields.
    ProxyV0 {
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        grandparent_qc: QuorumCert,
        /// Round of the most recent primary proof (QC/TC) in this block's ancestry
        last_primary_proof_round: Round,
        /// Primary consensus proof (QC or TC) attached at this cutting point
        primary_proof: Option<PrimaryConsensusProof>,
    },
}

impl OptBlockBody {
    pub fn author(&self) -> &Author {
        match self {
            OptBlockBody::V0 { author, .. } | OptBlockBody::ProxyV0 { author, .. } => author,
        }
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        match self {
            OptBlockBody::V0 { validator_txns, .. }
            | OptBlockBody::ProxyV0 { validator_txns, .. } => Some(validator_txns),
        }
    }

    pub fn payload(&self) -> &Payload {
        match self {
            OptBlockBody::V0 { payload, .. } | OptBlockBody::ProxyV0 { payload, .. } => payload,
        }
    }

    pub fn grandparent_qc(&self) -> &QuorumCert {
        match self {
            OptBlockBody::V0 { grandparent_qc, .. }
            | OptBlockBody::ProxyV0 { grandparent_qc, .. } => grandparent_qc,
        }
    }

    pub fn last_primary_proof_round(&self) -> Option<Round> {
        match self {
            OptBlockBody::V0 { .. } => None,
            OptBlockBody::ProxyV0 { last_primary_proof_round, .. } => Some(*last_primary_proof_round),
        }
    }

    pub fn primary_proof(&self) -> Option<&PrimaryConsensusProof> {
        match self {
            OptBlockBody::V0 { .. } => None,
            OptBlockBody::ProxyV0 { primary_proof, .. } => primary_proof.as_ref(),
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
    /// Proxy regular block — extends V0 with primary consensus linkage fields.
    ProxyV0 {
        validator_txns: Vec<ValidatorTransaction>,
        payload: Payload,
        author: Author,
        failed_authors: Vec<(Round, Author)>,
        /// Round of the most recent primary proof (QC/TC) in this block's ancestry
        last_primary_proof_round: Round,
        /// Primary consensus proof (QC or TC) attached at this cutting point
        primary_proof: Option<PrimaryConsensusProof>,
    },
}

impl ProposalExt {
    pub fn author(&self) -> &Author {
        match self {
            ProposalExt::V0 { author, .. } | ProposalExt::ProxyV0 { author, .. } => author,
        }
    }

    pub fn failed_authors(&self) -> &Vec<(Round, Author)> {
        match self {
            ProposalExt::V0 { failed_authors, .. }
            | ProposalExt::ProxyV0 { failed_authors, .. } => failed_authors,
        }
    }

    pub fn validator_txns(&self) -> Option<&Vec<ValidatorTransaction>> {
        match self {
            ProposalExt::V0 { validator_txns, .. }
            | ProposalExt::ProxyV0 { validator_txns, .. } => Some(validator_txns),
        }
    }

    pub fn payload(&self) -> Option<&Payload> {
        match self {
            ProposalExt::V0 { payload, .. } | ProposalExt::ProxyV0 { payload, .. } => {
                Some(payload)
            },
        }
    }

    pub fn last_primary_proof_round(&self) -> Option<Round> {
        match self {
            ProposalExt::V0 { .. } => None,
            ProposalExt::ProxyV0 { last_primary_proof_round, .. } => Some(*last_primary_proof_round),
        }
    }

    pub fn primary_proof(&self) -> Option<&PrimaryConsensusProof> {
        match self {
            ProposalExt::V0 { .. } => None,
            ProposalExt::ProxyV0 { primary_proof, .. } => primary_proof.as_ref(),
        }
    }
}
