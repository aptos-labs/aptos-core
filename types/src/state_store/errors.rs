// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
