// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::NetworkError, protocols::network::RpcError};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
pub enum Error {
    #[error("Network error encountered: {0}")]
    NetworkError(String),
    #[error("Rpc error encountered: {0}")]
    RpcError(String),
    #[error("Unexpected error encountered: {0}")]
    UnexpectedError(String),
}

impl From<anyhow::Error> for Error {
    fn from(error: anyhow::Error) -> Self {
        Error::UnexpectedError(error.to_string())
    }
}

impl From<NetworkError> for Error {
    fn from(error: NetworkError) -> Self {
        Error::NetworkError(error.to_string())
    }
}

impl From<RpcError> for Error {
    fn from(error: RpcError) -> Self {
        Error::RpcError(error.to_string())
    }
}
