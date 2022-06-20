// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{types, types::ErrorDetails};
use hex::FromHexError;
use move_deps::move_core_types::account_address::AccountAddressParseError;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use warp::{http::StatusCode, reply::Reply};

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Deserialize, Serialize, Error)]
pub enum ApiError {
    #[error("Aptos error {0}")]
    AptosError(String),
    #[error("bad block request")]
    BadBlockRequest,
    #[error("bad network")]
    BadNetwork,
    #[error("deserialization failed: {0}")]
    DeserializationFailed(String),
    #[error("bad transfer operations")]
    BadTransferOperations(String),
    #[error("account not found")]
    AccountNotFound,
    #[error("bad signature")]
    BadSignature,
    #[error("bad signature type")]
    BadSignatureType,
    #[error("bad transaction script")]
    BadTransactionScript,
    #[error("bad transaction payload")]
    BadTransactionPayload,
    #[error("bad coin")]
    BadCoin,
    #[error("bad signature count")]
    BadSignatureCount,
    #[error("historic balances unsupported")]
    HistoricBalancesUnsupported,
    #[error("node is offline")]
    NodeIsOffline,
}

impl ApiError {
    pub fn all() -> Vec<ApiError> {
        use ApiError::*;
        vec![
            AptosError(String::new()),
            BadBlockRequest,
            BadNetwork,
            DeserializationFailed(String::new()),
            BadTransferOperations(String::new()),
            AccountNotFound,
            BadSignature,
            BadSignatureType,
            BadTransactionScript,
            BadTransactionPayload,
            BadCoin,
            BadSignatureCount,
            HistoricBalancesUnsupported,
            NodeIsOffline,
        ]
    }

    pub fn code(&self) -> u64 {
        use ApiError::*;
        match self {
            AptosError(_) => 10,
            BadBlockRequest => 20,
            BadNetwork => 40,
            DeserializationFailed(_) => 50,
            BadTransferOperations(_) => 70,
            AccountNotFound => 80,
            BadSignature => 110,
            BadSignatureType => 120,
            BadTransactionScript => 130,
            BadTransactionPayload => 140,
            BadCoin => 150,
            BadSignatureCount => 160,
            HistoricBalancesUnsupported => 170,
            NodeIsOffline => 180,
        }
    }

    pub fn retriable(&self) -> bool {
        matches!(self, ApiError::AccountNotFound)
    }

    pub fn status_code(&self) -> StatusCode {
        use ApiError::*;
        match self {
            AccountNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::BAD_REQUEST,
        }
    }

    pub fn message(&self) -> String {
        let full = format!("{:?}", self);
        let parts: Vec<_> = full.split(':').collect();
        parts[0].to_string()
    }

    pub(crate) fn details(&self) -> ErrorDetails {
        let error = format!("{:?}", self);
        ErrorDetails { error }
    }

    pub fn deserialization_failed(type_: &str) -> ApiError {
        ApiError::DeserializationFailed(type_.to_string())
    }

    pub fn into_error(self) -> types::Error {
        self.into()
    }
}

impl From<ApiError> for types::Error {
    fn from(error: ApiError) -> Self {
        types::Error {
            message: error.message(),
            code: error.code(),
            retriable: error.retriable(),
            details: Some(error.details()),
            description: None,
        }
    }
}

impl From<&ApiError> for types::Error {
    fn from(error: &ApiError) -> Self {
        types::Error {
            message: error.message(),
            code: error.code(),
            retriable: error.retriable(),
            details: Some(error.details()),
            description: None,
        }
    }
}

impl From<AccountAddressParseError> for ApiError {
    fn from(err: AccountAddressParseError) -> Self {
        ApiError::AptosError(err.to_string())
    }
}

impl From<FromHexError> for ApiError {
    fn from(err: FromHexError) -> Self {
        ApiError::AptosError(err.to_string())
    }
}

impl From<bcs::Error> for ApiError {
    fn from(err: bcs::Error) -> Self {
        ApiError::AptosError(err.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::AptosError(err.to_string())
    }
}

impl warp::reject::Reject for ApiError {}

impl Reply for ApiError {
    fn into_response(self) -> warp::reply::Response {
        warp::reply::json(&self.into_error()).into_response()
    }
}
