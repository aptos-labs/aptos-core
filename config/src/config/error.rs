// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to sanitize the node config! Sanitizer: {0}, Error: {1}")]
    ConfigSanitizerFailed(String, String),
    #[error("Invariant violation: {0}")]
    InvariantViolation(String),
    #[error("Error accessing {0}: {1}")]
    IO(String, #[source] std::io::Error),
    #[error("Error (de)serializing {0}: {1}")]
    BCS(&'static str, #[source] bcs::Error),
    #[error("Error (de)serializing {0}: {1}")]
    Yaml(String, #[source] serde_yaml::Error),
    #[error("Config is missing expected value: {0}")]
    Missing(&'static str),
    #[error("Unexpected error: {0}")]
    Unexpected(String),
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Error::Unexpected(error.to_string())
    }
}
