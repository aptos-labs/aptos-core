// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
/// Different reasons for executor service fails to execute a block.
pub enum Error {
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

impl From<bcs::Error> for Error {
    fn from(error: bcs::Error) -> Self {
        Self::SerializationError(format!("{}", error))
    }
}

impl From<velor_secure_net::Error> for Error {
    fn from(error: velor_secure_net::Error) -> Self {
        Self::InternalError(error.to_string())
    }
}
