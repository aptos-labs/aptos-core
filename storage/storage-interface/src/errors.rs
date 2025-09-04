// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines error types used by `VelorDB`.
use velor_types::state_store::errors::StateViewError;
use std::sync::mpsc::RecvError;
use thiserror::Error;

/// This enum defines errors commonly used among `VelorDB` APIs.
#[derive(Clone, Debug, Error)]
pub enum VelorDbError {
    /// A requested item is not found.
    #[error("{0} not found.")]
    NotFound(String),
    /// Requested too many items.
    #[error("Too many items requested: at least {0} requested, max is {1}")]
    TooManyRequested(u64, u64),
    #[error("Missing state root node at version {0}, probably pruned.")]
    MissingRootError(u64),
    /// Other non-classified error.
    #[error("VelorDB Other Error: {0}")]
    Other(String),
    #[error("VelorDB RocksDb Error: {0}")]
    RocksDbIncompleteResult(String),
    #[error("VelorDB RocksDB Error: {0}")]
    OtherRocksDbError(String),
    #[error("VelorDB bcs Error: {0}")]
    BcsError(String),
    #[error("VelorDB IO Error: {0}")]
    IoError(String),
    #[error("VelorDB Recv Error: {0}")]
    RecvError(String),
    #[error("VelorDB ParseInt Error: {0}")]
    ParseIntError(String),
}

impl From<anyhow::Error> for VelorDbError {
    fn from(error: anyhow::Error) -> Self {
        Self::Other(format!("{}", error))
    }
}

impl From<bcs::Error> for VelorDbError {
    fn from(error: bcs::Error) -> Self {
        Self::BcsError(format!("{}", error))
    }
}

impl From<RecvError> for VelorDbError {
    fn from(error: RecvError) -> Self {
        Self::RecvError(format!("{}", error))
    }
}

impl From<std::io::Error> for VelorDbError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(format!("{}", error))
    }
}

impl From<std::num::ParseIntError> for VelorDbError {
    fn from(error: std::num::ParseIntError) -> Self {
        Self::Other(format!("{}", error))
    }
}

impl From<VelorDbError> for StateViewError {
    fn from(error: VelorDbError) -> Self {
        match error {
            VelorDbError::NotFound(msg) => StateViewError::NotFound(msg),
            VelorDbError::Other(msg) => StateViewError::Other(msg),
            _ => StateViewError::Other(format!("{}", error)),
        }
    }
}

impl From<StateViewError> for VelorDbError {
    fn from(error: StateViewError) -> Self {
        match error {
            StateViewError::NotFound(msg) => VelorDbError::NotFound(msg),
            StateViewError::Other(msg) => VelorDbError::Other(msg),
            StateViewError::BcsError(err) => VelorDbError::BcsError(err.to_string()),
        }
    }
}
