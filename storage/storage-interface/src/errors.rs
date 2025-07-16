// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines error types used by `AptosDB`.
use aptos_types::state_store::errors::StateViewError;
use std::sync::mpsc::RecvError;
use thiserror::Error;

/// This enum defines errors commonly used among `AptosDB` APIs.
#[derive(Clone, Debug, Error)]
pub enum AptosDbError {
    /// A requested item is not found.
    #[error("{0} not found.")]
    NotFound(String),
    /// Requested too many items.
    #[error("Too many items requested: at least {0} requested, max is {1}")]
    TooManyRequested(u64, u64),
    #[error("Missing state root node at version {0}, probably pruned.")]
    MissingRootError(u64),
    /// Other non-classified error.
    #[error("AptosDB Other Error: {0}")]
    Other(String),
    #[error("AptosDB RocksDb Error: {0}")]
    RocksDbIncompleteResult(String),
    #[error("AptosDB RocksDB Error: {0}")]
    OtherRocksDbError(String),
    #[error("AptosDB bcs Error: {0}")]
    BcsError(String),
    #[error("AptosDB IO Error: {0}")]
    IoError(String),
    #[error("AptosDB Recv Error: {0}")]
    RecvError(String),
    #[error("AptosDB ParseInt Error: {0}")]
    ParseIntError(String),
}

impl From<anyhow::Error> for AptosDbError {
    fn from(error: anyhow::Error) -> Self {
        Self::Other(format!("{}", error))
    }
}

impl From<bcs::Error> for AptosDbError {
    fn from(error: bcs::Error) -> Self {
        Self::BcsError(format!("{}", error))
    }
}

impl From<RecvError> for AptosDbError {
    fn from(error: RecvError) -> Self {
        Self::RecvError(format!("{}", error))
    }
}

impl From<std::io::Error> for AptosDbError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(format!("{}", error))
    }
}

impl From<std::num::ParseIntError> for AptosDbError {
    fn from(error: std::num::ParseIntError) -> Self {
        Self::Other(format!("{}", error))
    }
}

impl From<AptosDbError> for StateViewError {
    fn from(error: AptosDbError) -> Self {
        match error {
            AptosDbError::NotFound(msg) => StateViewError::NotFound(msg),
            AptosDbError::Other(msg) => StateViewError::Other(msg),
            _ => StateViewError::Other(format!("{}", error)),
        }
    }
}

impl From<StateViewError> for AptosDbError {
    fn from(error: StateViewError) -> Self {
        match error {
            StateViewError::NotFound(msg) => AptosDbError::NotFound(msg),
            StateViewError::Other(msg) => AptosDbError::Other(msg),
            StateViewError::BcsError(err) => AptosDbError::BcsError(err.to_string()),
        }
    }
}
