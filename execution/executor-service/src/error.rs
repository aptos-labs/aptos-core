// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
/// Different reasons for proposal rejection
pub enum Error {
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<aptos_secure_net::Error> for Error {
    fn from(error: aptos_secure_net::Error) -> Self {
        Self::InternalError(error.to_string())
    }
}
