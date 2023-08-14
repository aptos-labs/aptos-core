// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateviewError {
    #[error("{0} not found.")]
    NotFound(String),
    /// Other non-classified error.
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for StateviewError {
    fn from(error: anyhow::Error) -> Self {
        Self::Other(format!("{}", error))
    }
}
