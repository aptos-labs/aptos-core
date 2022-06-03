// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{types, types::ErrorDetails};
use thiserror::Error;
use warp::{http::StatusCode, reply::Reply};

#[derive(Debug, Error)]
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
    #[error("system time error: {0:?}")]
    SystemTimeError(#[from] std::time::SystemTimeError),
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
}

impl ApiError {
    pub fn code(&self) -> u64 {
        match self {
            ApiError::AptosError(_) => 10,
            ApiError::BadBlockRequest => 20,
            ApiError::BadNetwork => 40,
            ApiError::DeserializationFailed(_) => 50,
            ApiError::BadTransferOperations(_) => 70,
            ApiError::AccountNotFound => 80,
            ApiError::SystemTimeError(_) => 90,
            ApiError::BadSignature => 110,
            ApiError::BadSignatureType => 120,
            ApiError::BadTransactionScript => 130,
            ApiError::BadTransactionPayload => 140,
            ApiError::BadCoin => 150,
            ApiError::BadSignatureCount => 160,
            ApiError::HistoricBalancesUnsupported => 170,
        }
    }

    pub fn retriable(&self) -> bool {
        match self {
            ApiError::AptosError(_) => false,
            ApiError::BadBlockRequest => false,
            ApiError::BadNetwork => false,
            ApiError::DeserializationFailed(_) => false,
            ApiError::BadTransferOperations(_) => false,
            ApiError::AccountNotFound => true,
            ApiError::SystemTimeError(_) => true,
            ApiError::BadSignature => false,
            ApiError::BadSignatureType => false,
            ApiError::BadTransactionScript => false,
            ApiError::BadTransactionPayload => false,
            ApiError::BadCoin => false,
            ApiError::BadSignatureCount => false,
            ApiError::HistoricBalancesUnsupported => false,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::AptosError(_) => StatusCode::BAD_REQUEST,
            ApiError::BadBlockRequest => StatusCode::BAD_REQUEST,
            ApiError::BadNetwork => StatusCode::BAD_REQUEST,
            ApiError::DeserializationFailed(_) => StatusCode::BAD_REQUEST,
            ApiError::BadTransferOperations(_) => StatusCode::BAD_REQUEST,
            ApiError::AccountNotFound => StatusCode::NOT_FOUND,
            ApiError::SystemTimeError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::BadSignature => StatusCode::BAD_REQUEST,
            ApiError::BadSignatureType => StatusCode::BAD_REQUEST,
            ApiError::BadTransactionScript => StatusCode::BAD_REQUEST,
            ApiError::BadTransactionPayload => StatusCode::BAD_REQUEST,
            ApiError::BadCoin => StatusCode::BAD_REQUEST,
            ApiError::BadSignatureCount => StatusCode::BAD_REQUEST,
            ApiError::HistoricBalancesUnsupported => StatusCode::BAD_REQUEST,
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
        types::Error {
            message: self.message(),
            code: self.code(),
            retriable: self.retriable(),
            details: Some(self.details()),
        }
    }
}

impl warp::reject::Reject for ApiError {}

impl Reply for ApiError {
    fn into_response(self) -> warp::reply::Response {
        warp::reply::json(&self.into_error()).into_response()
    }
}
