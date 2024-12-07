// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::{Author, Payload, Round};
use aptos_types::validator_txn::ValidatorTransaction;
use serde::{Deserialize, Serialize};

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
    pub fn set_failed_authors(&mut self, failed_authors: Vec<(Round, Author)>) {
        match self {
            ProposalExt::V0 { failed_authors: fa, .. } => *fa = failed_authors,
        }
    }

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
