// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::types;
use hex::FromHexError;
use move_deps::move_core_types::account_address::AccountAddressParseError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use warp::{http::StatusCode, reply::Reply};

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Deserialize, Serialize, Error)]
pub enum ApiError {
    #[error("Aptos error: {0}")]
    AptosError(String),
    #[error("Must provide either hash or index but not both")]
    BlockParameterConflict,
    #[error("Transaction is pending")]
    TransactionIsPending,
    #[error("Network identifier doesn't match the supported network")]
    NetworkIdentifierMismatch,
    #[error("ChainId doesn't match the on-chain state")]
    ChainIdMismatch,
    #[error("Deserialization failed {0}")]
    DeserializationFailed(String),
    #[error("Transfer operations failed {0}")]
    InvalidTransferOperations(String),
    #[error("Account not found {0}")]
    AccountNotFound(String),
    #[error("Invalid signature type, only Ed25519 is supported")]
    InvalidSignatureType,
    #[error("Invalid max gas fees, only one native gas fee is allowed")]
    InvalidMaxGasFees,
    #[error("Invalid fee multiplier, only integers are allowed")]
    InvalidGasMultiplier,
    #[error("Operations don't map to a supported internal operation")]
    InvalidOperations,
    #[error("Missing payload metadata containing the internal transaction")]
    MissingPayloadMetadata,
    #[error("Unsupported currency {0}")]
    UnsupportedCurrency(String),
    #[error("Unsupported signature count {0}")]
    UnsupportedSignatureCount(usize),
    #[error("Node is offline, and this API is not supported in offline mode")]
    NodeIsOffline,
    #[error("Block is not yet complete, request will need to be retried")]
    BlockIncomplete,
}

impl ApiError {
    pub fn all() -> Vec<ApiError> {
        use ApiError::*;
        vec![
            AptosError(String::new()),
            TransactionIsPending,
            DeserializationFailed(String::new()),
            InvalidTransferOperations(String::new()),
            InvalidSignatureType,
            NodeIsOffline,
            BlockIncomplete,
            BlockParameterConflict,
            NetworkIdentifierMismatch,
            ChainIdMismatch,
            AccountNotFound(String::new()),
            InvalidMaxGasFees,
            InvalidGasMultiplier,
            InvalidOperations,
            MissingPayloadMetadata,
            UnsupportedCurrency(String::new()),
            UnsupportedSignatureCount(0),
        ]
    }

    pub fn code(&self) -> u64 {
        use ApiError::*;
        match self {
            AptosError(_) => 1,
            TransactionIsPending => 2,
            DeserializationFailed(_) => 3,
            InvalidTransferOperations(_) => 4,
            InvalidSignatureType => 5,
            NodeIsOffline => 6,
            BlockIncomplete => 7,
            BlockParameterConflict => 8,
            NetworkIdentifierMismatch => 9,
            ChainIdMismatch => 10,
            AccountNotFound(_) => 11,
            InvalidMaxGasFees => 12,
            InvalidGasMultiplier => 13,
            InvalidOperations => 14,
            MissingPayloadMetadata => 15,
            UnsupportedCurrency(_) => 16,
            UnsupportedSignatureCount(_) => 17,
        }
    }

    pub fn retriable(&self) -> bool {
        matches!(
            self,
            ApiError::AccountNotFound(_) | ApiError::BlockIncomplete
        )
    }

    pub fn status_code(&self) -> StatusCode {
        use ApiError::*;
        match self {
            AccountNotFound(_) => StatusCode::NOT_FOUND,
            BlockIncomplete => StatusCode::PRECONDITION_FAILED,
            NodeIsOffline => StatusCode::METHOD_NOT_ALLOWED,
            _ => StatusCode::BAD_REQUEST,
        }
    }

    pub fn message(&self) -> String {
        let full = format!("{:?}", self);
        let parts: Vec<_> = full.split(':').collect();
        parts[0].to_string()
    }

    pub fn deserialization_failed(type_: &str) -> ApiError {
        ApiError::DeserializationFailed(type_.to_string())
    }

    pub fn into_error(self) -> types::Error {
        (&self).into()
    }
}

impl From<&ApiError> for types::Error {
    fn from(error: &ApiError) -> Self {
        types::Error {
            message: error.message(),
            code: error.code(),
            retriable: error.retriable(),
            details: None,
            description: None,
        }
    }
}

impl From<AccountAddressParseError> for ApiError {
    fn from(err: AccountAddressParseError) -> Self {
        ApiError::DeserializationFailed(err.to_string())
    }
}

impl From<FromHexError> for ApiError {
    fn from(err: FromHexError) -> Self {
        ApiError::DeserializationFailed(err.to_string())
    }
}

impl From<bcs::Error> for ApiError {
    fn from(err: bcs::Error) -> Self {
        ApiError::DeserializationFailed(err.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::AptosError(err.to_string())
    }
}

impl From<std::num::ParseIntError> for ApiError {
    fn from(err: std::num::ParseIntError) -> Self {
        ApiError::DeserializationFailed(err.to_string())
    }
}

impl warp::reject::Reject for ApiError {}

impl Reply for ApiError {
    fn into_response(self) -> warp::reply::Response {
        warp::reply::json(&self.into_error()).into_response()
    }
}
