// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum Error {
    #[error("Invalid request received: {0}")]
    InvalidRequest(String),
    #[error("Storage error encountered: {0}")]
    StorageErrorEncountered(String),
    #[error("Too many invalid requests: {0}")]
    TooManyInvalidRequests(String),
    #[error("Unexpected error encountered: {0}")]
    UnexpectedErrorEncountered(String),
}

impl Error {
    /// Returns a summary label for the error type
    pub fn get_label(&self) -> &'static str {
        match self {
            Error::InvalidRequest(_) => "invalid_request",
            Error::StorageErrorEncountered(_) => "storage_error",
            Error::TooManyInvalidRequests(_) => "too_many_invalid_requests",
            Error::UnexpectedErrorEncountered(_) => "unexpected_error",
        }
    }
}

impl From<aptos_storage_service_types::responses::Error> for Error {
    fn from(error: aptos_storage_service_types::responses::Error) -> Self {
        Error::UnexpectedErrorEncountered(error.to_string())
    }
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Error::UnexpectedErrorEncountered(error.to_string())
    }
}

impl From<aptos_storage_interface::AptosDbError> for Error {
    fn from(error: aptos_storage_interface::AptosDbError) -> Self {
        Error::StorageErrorEncountered(error.to_string())
    }
}
