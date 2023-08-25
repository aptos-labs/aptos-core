// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

//! This module defines error types used by [`AptosDB`](crate::AptosDB).

use rocksdb;
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
    /// Other non-classified error.
    #[error("Other: {0}")]
    Other(String),
}

impl From<anyhow::Error> for AptosDbError {
    fn from(error: anyhow::Error) -> Self {
        Self::Other(format!("{}", error))
    }
}

impl From<bcs::Error> for AptosDbError {
    fn from(error: bcs::Error) -> Self {
        Self::Other(format!("{}", error))
    }
}

impl From<rocksdb::Error> for AptosDbError {
    fn from(error: rocksdb::Error) -> Self {
        Self::Other(format!("{}", error))
    }
}

impl From<RecvError> for AptosDbError {
    fn from(error: RecvError) -> Self {
        Self::Other(format!("{}", error))
    }
}

impl From<std::io::Error> for AptosDbError {
    fn from(error: std::io::Error) -> Self {
        Self::Other(format!("{}", error))
    }
}

impl From<std::num::ParseIntError> for AptosDbError {
    fn from(error: std::num::ParseIntError) -> Self {
        Self::Other(format!("{}", error))
    }
}
