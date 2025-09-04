// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_crypto::HashValue;
use velor_storage_interface::VelorDbError;
use velor_types::{state_store::errors::StateViewError, transaction::Version};
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use thiserror::Error;

#[derive(Debug, Deserialize, Error, PartialEq, Eq, Serialize, Clone)]
/// Different reasons for proposal rejection
pub enum ExecutorError {
    #[error("Cannot find speculation result for block id {0}")]
    BlockNotFound(HashValue),

    #[error("Cannot get data for batch id {0}")]
    DataNotFound(HashValue),

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

    #[error("request timeout")]
    CouldNotGetData,
}

impl From<anyhow::Error> for ExecutorError {
    fn from(error: anyhow::Error) -> Self {
        Self::InternalError {
            error: format!("{}", error),
        }
    }
}

impl From<VelorDbError> for ExecutorError {
    fn from(error: VelorDbError) -> Self {
        Self::InternalError {
            error: format!("{}", error),
        }
    }
}

impl From<StateViewError> for ExecutorError {
    fn from(error: StateViewError) -> Self {
        Self::InternalError {
            error: format!("{}", error),
        }
    }
}

impl From<bcs::Error> for ExecutorError {
    fn from(error: bcs::Error) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<velor_secure_net::Error> for ExecutorError {
    fn from(error: velor_secure_net::Error) -> Self {
        Self::InternalError {
            error: format!("{}", error),
        }
    }
}

impl ExecutorError {
    pub fn internal_err<E: Display>(e: E) -> Self {
        Self::InternalError {
            error: format!("{}", e),
        }
    }
}

pub type ExecutorResult<T> = Result<T, ExecutorError>;
