// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use poem_openapi::{Enum, Object};
use serde::Deserialize;
use std::convert::From;

use crate::move_types::U64;

/// This is the generic struct we use for all API errors, it contains a string
/// message and an Aptos API specific error code.
#[derive(Debug, Deserialize, Object)]
pub struct AptosError {
    pub message: String,
    pub error_code: Option<AptosErrorCode>,
    pub aptos_ledger_version: Option<U64>,
}

impl AptosError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            error_code: None,
            aptos_ledger_version: None,
        }
    }

    pub fn error_code(mut self, error_code: AptosErrorCode) -> Self {
        self.error_code = Some(error_code);
        self
    }

    pub fn aptos_ledger_version(mut self, ledger_version: u64) -> Self {
        self.aptos_ledger_version = Some(ledger_version.into());
        self
    }
}

impl From<anyhow::Error> for AptosError {
    fn from(error: anyhow::Error) -> Self {
        AptosError::new(format!("{:#}", error))
    }
}

/// These codes provide more granular error information beyond just the HTTP
/// status code of the response.
// Make sure the integer codes increment one by one.
#[derive(Debug, Deserialize, Enum)]
#[oai(rename_all = "snake_case")]
pub enum AptosErrorCode {
    /// The API failed to read from storage for this request, not because of a
    /// bad request, but because of some internal error.
    ReadFromStorageError = 1,

    /// The data we read from the DB was not valid BCS.
    InvalidBcsInStorageError = 2,

    /// We were unexpectedly unable to convert a Rust type to BCS.
    BcsSerializationError = 3,

    /// The start param given for paging is invalid.
    InvalidStartParam = 4,

    /// The limit param given for paging is invalid.
    InvalidLimitParam = 5,
}
