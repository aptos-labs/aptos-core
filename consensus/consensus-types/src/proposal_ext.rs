// Copyright © Aptos Foundation

use crate::common::{Author, Payload, Round};
use aptos_types::system_txn::SystemTransaction;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub enum ProposalExt {
    V0 {
        sys_txns: Vec<SystemTransaction>,
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

    pub fn sys_txns(&self) -> Option<&Vec<SystemTransaction>> {
        match self {
            ProposalExt::V0 { sys_txns, .. } => Some(sys_txns),
        }
    }
}
