// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{types, types::ErrorDetails};
use aptos_rest_client::{aptos_api_types::AptosErrorCode, error::RestError};
use hex::FromHexError;
use move_core_types::account_address::AccountAddressParseError;
use serde::{Deserialize, Serialize};
use std::fmt::Formatter;
use warp::{http::StatusCode, reply::Reply};

/// Result for Rosetta API errors
pub type ApiResult<T> = Result<T, ApiError>;

/// All Rosetta API errors.  Note that all details must be `Option<T>` to make it easier to list all
/// error messages in the `ApiError::all()` call required by the Rosetta spec.
#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub enum ApiError {
    TransactionIsPending,
    NetworkIdentifierMismatch,
    ChainIdMismatch,
    DeserializationFailed(Option<String>),
    InvalidTransferOperations(Option<&'static str>),
    InvalidSignatureType,
    InvalidMaxGasFees,
    MaxGasFeeTooLow(Option<String>),
    InvalidGasMultiplier,
    GasEstimationFailed(Option<String>),
    InvalidOperations(Option<String>),
    MissingPayloadMetadata,
    UnsupportedCurrency(Option<String>),
    UnsupportedSignatureCount(Option<usize>),
    NodeIsOffline,
    TransactionParseError(Option<String>),
    InternalError(Option<String>),
    CoinTypeFailedToBeFetched(Option<String>),

    // Below here are codes directly from the REST API
    AccountNotFound(Option<String>),
    ResourceNotFound(Option<String>),
    ModuleNotFound(Option<String>),
    StructFieldNotFound(Option<String>),
    VersionNotFound(Option<String>),
    TransactionNotFound(Option<String>),
    TableItemNotFound(Option<String>),
    BlockNotFound(Option<String>),
    StateValueNotFound(Option<String>),
    VersionPruned(Option<String>),
    BlockPruned(Option<String>),
    InvalidInput(Option<String>),
    InvalidTransactionUpdate(Option<String>),
    SequenceNumberTooOld(Option<String>),
    VmError(Option<String>),
    MempoolIsFull(Option<String>),
    RejectedByFilter(Option<String>),
    RateLimited(Option<String>),
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ApiError {}

/// The stable wire metadata for an [`ApiError`], produced by [`ApiError::info`].
struct ErrorInfo {
    code: u32,
    retriable: bool,
    message: &'static str,
}

impl ApiError {
    /// Returns every single API errors so the messages can be returned
    pub fn all() -> Vec<ApiError> {
        use ApiError::*;
        vec![
            TransactionIsPending,
            NetworkIdentifierMismatch,
            ChainIdMismatch,
            DeserializationFailed(None),
            InvalidTransferOperations(None),
            InvalidSignatureType,
            InvalidMaxGasFees,
            MaxGasFeeTooLow(None),
            InvalidGasMultiplier,
            GasEstimationFailed(None),
            InvalidOperations(None),
            MissingPayloadMetadata,
            UnsupportedCurrency(None),
            UnsupportedSignatureCount(None),
            NodeIsOffline,
            TransactionParseError(None),
            InternalError(None),
            CoinTypeFailedToBeFetched(None),
            AccountNotFound(None),
            ResourceNotFound(None),
            ModuleNotFound(None),
            StructFieldNotFound(None),
            VersionNotFound(None),
            TransactionNotFound(None),
            TableItemNotFound(None),
            BlockNotFound(None),
            StateValueNotFound(None),
            VersionPruned(None),
            BlockPruned(None),
            InvalidInput(None),
            InvalidTransactionUpdate(None),
            SequenceNumberTooOld(None),
            VmError(None),
            MempoolIsFull(None),
            RejectedByFilter(None),
            RateLimited(None),
        ]
    }

    /// The stable, wire-facing metadata for an error: its numeric `code`, whether
    /// upstreams may `retriable`-retry, and the fixed `message`.
    ///
    /// This is the SINGLE SOURCE OF TRUTH (BC-3): `code()`, `retriable()`, and
    /// `message()` are thin accessors over this table so they can never disagree.
    /// Codes are permanent and must not change; see docs/SPEC_DEVIATIONS.md §9.
    fn info(&self) -> ErrorInfo {
        use ApiError::*;
        // (code, retriable, message)
        let (code, retriable, message) = match self {
            TransactionIsPending => (1, false, "Transaction is pending"),
            NetworkIdentifierMismatch => (2, false, "Network identifier doesn't match"),
            ChainIdMismatch => (3, false, "Chain Id doesn't match"),
            DeserializationFailed(_) => (4, false, "Deserialization failed"),
            InvalidTransferOperations(_) => (5, false, "Invalid operations for a transfer"),
            InvalidSignatureType => (6, false, "Invalid signature type"),
            InvalidMaxGasFees => (7, false, "Invalid max gas fee"),
            MaxGasFeeTooLow(_) => (
                8,
                false,
                "Max fee is lower than the estimated cost of the transaction",
            ),
            InvalidGasMultiplier => (9, false, "Invalid gas multiplier"),
            InvalidOperations(_) => (10, false, "Invalid operations"),
            MissingPayloadMetadata => (11, false, "Payload metadata is missing"),
            UnsupportedCurrency(_) => (12, false, "Currency is unsupported"),
            UnsupportedSignatureCount(_) => (13, false, "Number of signatures is not supported"),
            // BC-1: fixed from "...because he's offline".
            NodeIsOffline => (14, false, "This API is unavailable because the node is offline"),
            TransactionParseError(_) => (15, false, "Transaction failed to parse"),
            GasEstimationFailed(_) => (16, true, "Gas estimation failed"),
            InternalError(_) => (17, false, "Internal error"),
            AccountNotFound(_) => (18, true, "Account not found"),
            ResourceNotFound(_) => (19, false, "Resource not found"),
            ModuleNotFound(_) => (20, false, "Module not found"),
            StructFieldNotFound(_) => (21, false, "Struct field not found"),
            VersionNotFound(_) => (22, false, "Version not found"),
            TransactionNotFound(_) => (23, false, "Transaction not found"),
            TableItemNotFound(_) => (24, false, "Table item not found"),
            // BC-1: fixed from "Block is missing events".
            BlockNotFound(_) => (25, true, "Block not found"),
            VersionPruned(_) => (26, false, "Version pruned"),
            BlockPruned(_) => (27, false, "Block pruned"),
            InvalidInput(_) => (28, false, "Invalid input"),
            InvalidTransactionUpdate(_) => (
                29,
                false,
                "Invalid transaction update.  Can only update gas unit price",
            ),
            SequenceNumberTooOld(_) => (
                30,
                false,
                "Sequence number too old.  Please create a new transaction with an updated sequence number",
            ),
            VmError(_) => (31, false, "Transaction submission failed due to VM error"),
            MempoolIsFull(_) => (32, true, "Mempool is full all accounts"),
            // BC-1: fixed from "Faileed to retrieve...".
            CoinTypeFailedToBeFetched(_) => (
                33,
                true,
                "Failed to retrieve the coin type information, please retry",
            ),
            StateValueNotFound(_) => (34, false, "StateValue not found."),
            RejectedByFilter(_) => (
                35,
                false,
                "Transaction was rejected by the transaction filter",
            ),
            RateLimited(_) => (36, true, "Rate limited"),
        };
        ErrorInfo {
            code,
            retriable,
            message,
        }
    }

    /// The stable numeric code for this error (permanent; see SPEC_DEVIATIONS §9).
    pub fn code(&self) -> u32 {
        self.info().code
    }

    /// Whether Rosetta upstreams may retry.  Only temporary/state blockers are
    /// retriable (e.g. an account not yet created, a full mempool).
    pub fn retriable(&self) -> bool {
        self.info().retriable
    }

    /// All Rosetta errors are HTTP 500; `retriable()` says whether a retry helps.
    pub fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    /// The fixed, wire-facing message for this error.  Stable per code.
    pub fn message(&self) -> &'static str {
        self.info().message
    }

    /// Details are optional, but give more details for each error message
    pub fn details(self) -> Option<ErrorDetails> {
        match self {
            ApiError::DeserializationFailed(inner) => inner,
            ApiError::InvalidTransferOperations(inner) => inner.map(|inner| inner.to_string()),
            ApiError::UnsupportedCurrency(inner) => inner,
            ApiError::UnsupportedSignatureCount(inner) => inner.map(|inner| inner.to_string()),
            ApiError::TransactionParseError(inner) => inner,
            ApiError::InvalidOperations(inner) => inner,
            ApiError::InternalError(inner) => inner,
            ApiError::CoinTypeFailedToBeFetched(inner) => inner,
            ApiError::AccountNotFound(inner) => inner,
            ApiError::ResourceNotFound(inner) => inner,
            ApiError::ModuleNotFound(inner) => inner,
            ApiError::StructFieldNotFound(inner) => inner,
            ApiError::VersionNotFound(inner) => inner,
            ApiError::TransactionNotFound(inner) => inner,
            ApiError::TableItemNotFound(inner) => inner,
            ApiError::BlockNotFound(inner) => inner,
            ApiError::VersionPruned(inner) => inner,
            ApiError::BlockPruned(inner) => inner,
            ApiError::InvalidInput(inner) => inner,
            ApiError::InvalidTransactionUpdate(inner) => inner,
            ApiError::SequenceNumberTooOld(inner) => inner,
            ApiError::VmError(inner) => inner,
            ApiError::MempoolIsFull(inner) => inner,
            ApiError::GasEstimationFailed(inner) => inner,
            ApiError::MaxGasFeeTooLow(inner) => inner,
            _ => None,
        }
        .map(|details| ErrorDetails { details })
    }

    pub fn deserialization_failed(type_: &str) -> ApiError {
        ApiError::DeserializationFailed(Some(type_.to_string()))
    }

    /// Converts API Error into the wire representation
    pub fn into_error(self) -> types::Error {
        self.into()
    }
}

impl From<ApiError> for types::Error {
    fn from(error: ApiError) -> Self {
        let message = error.message().to_string();
        let code = error.code();
        let retriable = error.retriable();
        let details = error.details();
        types::Error {
            message,
            code,
            retriable,
            details,
        }
    }
}

// Converts Node API errors to Rosetta API errors
impl From<RestError> for ApiError {
    fn from(err: RestError) -> Self {
        match err {
            RestError::Api(err) => match err.error.error_code {
                AptosErrorCode::AccountNotFound => {
                    ApiError::AccountNotFound(Some(err.error.message))
                },
                AptosErrorCode::ResourceNotFound => {
                    ApiError::ResourceNotFound(Some(err.error.message))
                },
                AptosErrorCode::ModuleNotFound => ApiError::ModuleNotFound(Some(err.error.message)),
                AptosErrorCode::StructFieldNotFound => {
                    ApiError::StructFieldNotFound(Some(err.error.message))
                },
                AptosErrorCode::VersionNotFound => {
                    ApiError::VersionNotFound(Some(err.error.message))
                },
                AptosErrorCode::TransactionNotFound => {
                    ApiError::TransactionNotFound(Some(err.error.message))
                },
                AptosErrorCode::TableItemNotFound => {
                    ApiError::TableItemNotFound(Some(err.error.message))
                },
                AptosErrorCode::BlockNotFound => ApiError::BlockNotFound(Some(err.error.message)),
                AptosErrorCode::StateValueNotFound => {
                    ApiError::StateValueNotFound(Some(err.error.message))
                },
                AptosErrorCode::VersionPruned => ApiError::VersionPruned(Some(err.error.message)),
                AptosErrorCode::BlockPruned => ApiError::BlockPruned(Some(err.error.message)),
                AptosErrorCode::InvalidInput => ApiError::InvalidInput(Some(err.error.message)),
                // BC-2: was incorrectly mapped to InvalidInput (code 28); now maps
                // to the matching InvalidTransactionUpdate (code 29).
                AptosErrorCode::InvalidTransactionUpdate => {
                    ApiError::InvalidTransactionUpdate(Some(err.error.message))
                },
                AptosErrorCode::SequenceNumberTooOld => {
                    ApiError::SequenceNumberTooOld(Some(err.error.message))
                },
                AptosErrorCode::VmError => ApiError::VmError(Some(err.error.message)),
                AptosErrorCode::RejectedByFilter => {
                    ApiError::RejectedByFilter(Some(err.error.message))
                },
                AptosErrorCode::HealthCheckFailed => {
                    ApiError::InternalError(Some(err.error.message))
                },
                AptosErrorCode::MempoolIsFull => ApiError::MempoolIsFull(Some(err.error.message)),
                AptosErrorCode::RateLimited => ApiError::RateLimited(Some(err.error.message)),
                AptosErrorCode::WebFrameworkError => {
                    ApiError::InternalError(Some(err.error.message))
                },
                AptosErrorCode::BcsNotSupported => ApiError::InvalidInput(Some(err.error.message)),
                AptosErrorCode::InternalError => ApiError::InternalError(Some(err.error.message)),
                AptosErrorCode::ApiDisabled => ApiError::InternalError(Some(err.error.message)),
            },
            RestError::Bcs(_) => ApiError::DeserializationFailed(None),
            RestError::Json(_) => ApiError::DeserializationFailed(None),
            RestError::Http(status_code, err) => ApiError::InternalError(Some(format!(
                "Failed internal API call with HTTP code {}: {:#}",
                status_code, err
            ))),
            RestError::UrlParse(err) => ApiError::InternalError(Some(err.to_string())),
            RestError::Timeout(err) => ApiError::InternalError(Some(err.to_string())),
            RestError::Unknown(err) => ApiError::InternalError(Some(err.to_string())),
        }
    }
}

impl From<AccountAddressParseError> for ApiError {
    fn from(err: AccountAddressParseError) -> Self {
        ApiError::DeserializationFailed(Some(err.to_string()))
    }
}

impl From<FromHexError> for ApiError {
    fn from(err: FromHexError) -> Self {
        ApiError::DeserializationFailed(Some(err.to_string()))
    }
}

impl From<bcs::Error> for ApiError {
    fn from(err: bcs::Error) -> Self {
        ApiError::DeserializationFailed(Some(err.to_string()))
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        ApiError::InternalError(Some(err.to_string()))
    }
}

impl From<std::num::ParseIntError> for ApiError {
    fn from(err: std::num::ParseIntError) -> Self {
        ApiError::DeserializationFailed(Some(err.to_string()))
    }
}

// Must implement to ensure rejections are provided when returning errors
impl warp::reject::Reject for ApiError {}

impl Reply for ApiError {
    fn into_response(self) -> warp::reply::Response {
        warp::reply::json(&self.into_error()).into_response()
    }
}
