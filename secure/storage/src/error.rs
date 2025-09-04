// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::io;
use thiserror::Error;

#[derive(Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum Error {
    #[error("Entropy error: {0}")]
    EntropyError(String),
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Key already exists: {0}")]
    KeyAlreadyExists(String),
    #[error("Key not set: {0}")]
    KeyNotSet(String),
    #[error("Permission denied")]
    PermissionDenied,
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Key version not found, key name: {0}, version: {1}")]
    KeyVersionNotFound(String, String),
}

impl From<base64::DecodeError> for Error {
    fn from(error: base64::DecodeError) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<chrono::format::ParseError> for Error {
    fn from(error: chrono::format::ParseError) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::InternalError(format!("{}", error))
    }
}

impl From<bcs::Error> for Error {
    fn from(error: bcs::Error) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<velor_vault_client::Error> for Error {
    fn from(error: velor_vault_client::Error) -> Self {
        match error {
            velor_vault_client::Error::NotFound(_, key) => Self::KeyNotSet(key),
            velor_vault_client::Error::HttpError(403, _, _) => Self::PermissionDenied,
            _ => Self::InternalError(format!("{}", error)),
        }
    }
}
