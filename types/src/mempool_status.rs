// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::unit_arg)]

use anyhow::Result;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use std::{convert::TryFrom, fmt};

/// A `MempoolStatus` is represented as a required status code that is semantic coupled with an optional sub status and message.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
#[cfg_attr(any(test, feature = "fuzzing"), proptest(no_params))]
pub struct MempoolStatus {
    /// insertion status code
    pub code: MempoolStatusCode,
    /// optional message
    pub message: String,
}

impl MempoolStatus {
    pub fn new(code: MempoolStatusCode) -> Self {
        Self {
            code,
            message: "".to_string(),
        }
    }

    /// Adds a message to the Mempool status.
    pub fn with_message(mut self, message: String) -> Self {
        self.message = message;
        self
    }
}

impl fmt::Display for MempoolStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.code)?;
        if !self.message.is_empty() {
            write!(f, " - {}", &self.message)?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
#[repr(u64)]
pub enum MempoolStatusCode {
    // Transaction was accepted by Mempool
    Accepted = 0,
    // Sequence number is old, etc.
    InvalidSeqNumber = 1,
    // Mempool is full (reached max global capacity)
    MempoolIsFull = 2,
    // Account reached max capacity per account
    TooManyTransactions = 3,
    // Invalid update. Only gas price increase is allowed
    InvalidUpdate = 4,
    // transaction didn't pass vm_validation
    VmError = 5,
    UnknownStatus = 6,
    // The transaction filter has rejected the transaction
    RejectedByFilter = 7,
}

impl TryFrom<u64> for MempoolStatusCode {
    type Error = &'static str;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(MempoolStatusCode::Accepted),
            1 => Ok(MempoolStatusCode::InvalidSeqNumber),
            2 => Ok(MempoolStatusCode::MempoolIsFull),
            3 => Ok(MempoolStatusCode::TooManyTransactions),
            4 => Ok(MempoolStatusCode::InvalidUpdate),
            5 => Ok(MempoolStatusCode::VmError),
            6 => Ok(MempoolStatusCode::UnknownStatus),
            7 => Ok(MempoolStatusCode::RejectedByFilter),
            _ => Err("invalid StatusCode"),
        }
    }
}

impl From<MempoolStatusCode> for u64 {
    fn from(status: MempoolStatusCode) -> u64 {
        status as u64
    }
}

impl fmt::Display for MempoolStatusCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
