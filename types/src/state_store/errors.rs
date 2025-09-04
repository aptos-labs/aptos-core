// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateViewError {
    #[error("{0} not found.")]
    NotFound(String),
    /// Other non-classified error.
    #[error("{0}")]
    Other(String),
    #[error(transparent)]
    BcsError(#[from] bcs::Error),
}

impl From<anyhow::Error> for StateViewError {
    fn from(error: anyhow::Error) -> Self {
        Self::Other(format!("{}", error))
    }
}
