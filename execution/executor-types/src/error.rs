// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use thiserror::Error;

use aptos_crypto::HashValue;
use aptos_types::transaction::Version;

#[derive(Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
/// Different reasons for proposal rejection
pub enum Error {
    #[error("Cannot find speculation result for block id {0}")]
    BlockNotFound(HashValue),

    #[error(
        "Bad num_txns_to_commit. first version {}, num to commit: {}, target version: {}",
        first_version,
        to_commit,
        target_version
    )]
    BadNumTxnsToCommit {
        first_version: Version,
        to_commit: usize,
        target_version: Version,
    },

    #[error("Internal error: {:?}", error)]
    InternalError { error: String },

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Received Empty Blocks")]
    EmptyBlocks,
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Self::InternalError {
            error: format!("{}", error),
        }
    }
}

impl From<bcs::Error> for Error {
    fn from(error: bcs::Error) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<aptos_secure_net::Error> for Error {
    fn from(error: aptos_secure_net::Error) -> Self {
        Self::InternalError {
            error: format!("{}", error),
        }
    }
}
