// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A pepper service error (e.g., for bad API requests, internal errors, etc.)
#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum PepperServiceError {
    #[error("Bad request error: {0}")]
    BadRequest(String),
    #[error("Internal service error: {0}")]
    InternalError(String),
    #[error("Unexpected error: {0}")]
    UnexpectedError(String),
}

impl From<tokio::task::JoinError> for PepperServiceError {
    fn from(error: tokio::task::JoinError) -> PepperServiceError {
        PepperServiceError::UnexpectedError(format!("JoinError: {:?}", error))
    }
}
