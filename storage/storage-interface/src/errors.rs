// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines error types used by [`AptosDB`](crate::AptosDB).
use aptos_types::state_store::errors::StateviewError;
use std::sync::mpsc::RecvError;
use thiserror::Error;

/// This enum defines errors commonly used among [`AptosDB`](crate::AptosDB) APIs.
#[derive(Debug, Error)]
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
    #[error("AptosDB RocksDB Error: {0}")]
    RocksDbError(String),
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

impl From<rocksdb::Error> for AptosDbError {
    fn from(error: rocksdb::Error) -> Self {
        Self::RocksDbError(format!("{}", error))
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

impl From<AptosDbError> for StateviewError {
    fn from(error: AptosDbError) -> Self {
        match error {
            AptosDbError::NotFound(msg) => StateviewError::NotFound(msg),
            AptosDbError::Other(msg) => StateviewError::Other(msg),
            _ => StateviewError::Other(format!("{}", error)),
        }
    }
}

impl From<StateviewError> for AptosDbError {
    fn from(error: StateviewError) -> Self {
        match error {
            StateviewError::NotFound(msg) => AptosDbError::NotFound(msg),
            StateviewError::Other(msg) => AptosDbError::Other(msg),
        }
    }
}
