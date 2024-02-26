// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::NetworkError, protocols::network::RpcError};
use thiserror::Error;

// #[derive(Clone, Debug, Deserialize, Error, PartialEq, Eq, Serialize)]
#[derive(Debug, Error)]
pub enum Error {
    #[error("Network error encountered: {0}")]
    NetworkError(NetworkError),
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
        Error::NetworkError(error)
    }
}

impl From<RpcError> for Error {
    fn from(error: RpcError) -> Self {
        Error::RpcError(error.to_string())
    }
}
