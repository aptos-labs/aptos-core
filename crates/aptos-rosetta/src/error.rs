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
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for ApiError {}

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
        ]
    }

    /// All errors are required to have a code.  These are just in order that they were added, and no specific grouping.
    pub fn code(&self) -> u32 {
        use ApiError::*;
        match self {
            TransactionIsPending => 1,
            NetworkIdentifierMismatch => 2,
            ChainIdMismatch => 3,
            DeserializationFailed(_) => 4,
            InvalidTransferOperations(_) => 5,
            InvalidSignatureType => 6,
            InvalidMaxGasFees => 7,
            MaxGasFeeTooLow(_) => 8,
            InvalidGasMultiplier => 9,
            InvalidOperations(_) => 10,
            MissingPayloadMetadata => 11,
            UnsupportedCurrency(_) => 12,
            UnsupportedSignatureCount(_) => 13,
            NodeIsOffline => 14,
            TransactionParseError(_) => 15,
            GasEstimationFailed(_) => 16,
            InternalError(_) => 17,
            AccountNotFound(_) => 18,
            ResourceNotFound(_) => 19,
            ModuleNotFound(_) => 20,
            StructFieldNotFound(_) => 21,
            VersionNotFound(_) => 22,
            TransactionNotFound(_) => 23,
            TableItemNotFound(_) => 24,
            BlockNotFound(_) => 25,
            VersionPruned(_) => 26,
            BlockPruned(_) => 27,
            InvalidInput(_) => 28,
            InvalidTransactionUpdate(_) => 29,
            SequenceNumberTooOld(_) => 30,
            VmError(_) => 31,
            MempoolIsFull(_) => 32,
            CoinTypeFailedToBeFetched(_) => 33,
            StateValueNotFound(_) => 34,
            RejectedByFilter(_) => 35,
        }
    }

    /// Retriable errors will allow for Rosetta upstreams to retry.  These are only for temporary
    /// state blockers.  Note, there is a possibility that some of these could be retriable forever (e.g. an account is never created).
    pub fn retriable(&self) -> bool {
        use ApiError::*;
        matches!(
            self,
            AccountNotFound(_)
                | BlockNotFound(_)
                | MempoolIsFull(_)
                | GasEstimationFailed(_)
                | CoinTypeFailedToBeFetched(_)
        )
    }

    /// All Rosetta errors must be 500s (and retriable tells you if it's actually retriable)
    pub fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }

    /// This value must be fixed, so it's all static strings
    pub fn message(&self) -> &'static str {
        match self {
            ApiError::TransactionIsPending => "Transaction is pending",
            ApiError::NetworkIdentifierMismatch => "Network identifier doesn't match",
            ApiError::ChainIdMismatch => "Chain Id doesn't match",
            ApiError::DeserializationFailed(_) => "Deserialization failed",
            ApiError::InvalidTransferOperations(_) => "Invalid operations for a transfer",
            ApiError::AccountNotFound(_) => "Account not found",
            ApiError::InvalidSignatureType => "Invalid signature type",
            ApiError::InvalidMaxGasFees => "Invalid max gas fee",
            ApiError::MaxGasFeeTooLow(_) => "Max fee is lower than the estimated cost of the transaction",
            ApiError::InvalidGasMultiplier => "Invalid gas multiplier",
            ApiError::InvalidOperations(_) => "Invalid operations",
            ApiError::MissingPayloadMetadata => "Payload metadata is missing",
            ApiError::UnsupportedCurrency(_) => "Currency is unsupported",
            ApiError::UnsupportedSignatureCount(_) => "Number of signatures is not supported",
            ApiError::NodeIsOffline => "This API is unavailable for the node because he's offline",
            ApiError::BlockNotFound(_) => "Block is missing events",
            ApiError::StateValueNotFound(_) => "StateValue not found.",
            ApiError::TransactionParseError(_) => "Transaction failed to parse",
            ApiError::InternalError(_) => "Internal error",
            ApiError::CoinTypeFailedToBeFetched(_) => "Faileed to retrieve the coin type information, please retry",
            ApiError::ResourceNotFound(_) => "Resource not found",
            ApiError::ModuleNotFound(_) => "Module not found",
            ApiError::StructFieldNotFound(_) => "Struct field not found",
            ApiError::VersionNotFound(_) => "Version not found",
            ApiError::TransactionNotFound(_) => "Transaction not found",
            ApiError::TableItemNotFound(_) => "Table item not found",
            ApiError::VersionPruned(_) => "Version pruned",
            ApiError::BlockPruned(_) => "Block pruned",
            ApiError::InvalidInput(_) => "Invalid input",
            ApiError::InvalidTransactionUpdate(_) => "Invalid transaction update.  Can only update gas unit price",
            ApiError::SequenceNumberTooOld(_) => "Sequence number too old.  Please create a new transaction with an updated sequence number",
            ApiError::VmError(_) => "Transaction submission failed due to VM error",
            ApiError::MempoolIsFull(_) => "Mempool is full all accounts",
            ApiError::GasEstimationFailed(_) => "Gas estimation failed",
            ApiError::RejectedByFilter(_) => "Transaction was rejected by the transaction filter",
        }
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
                AptosErrorCode::InvalidTransactionUpdate => {
                    ApiError::InvalidInput(Some(err.error.message))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_error_codes_are_unique() {
        let all_errors = ApiError::all();
        let mut codes = HashSet::new();
        for error in &all_errors {
            assert!(
                codes.insert(error.code()),
                "Duplicate error code: {}",
                error.code()
            );
        }
    }

    #[test]
    fn test_all_errors_have_codes() {
        let all_errors = ApiError::all();
        for error in &all_errors {
            assert!(error.code() > 0, "Error code must be > 0");
            assert!(error.code() <= 35, "Unexpected error code: {}", error.code());
        }
    }

    #[test]
    fn test_all_errors_have_messages() {
        let all_errors = ApiError::all();
        for error in &all_errors {
            let message = error.message();
            assert!(!message.is_empty(), "Error message must not be empty");
        }
    }

    #[test]
    fn test_all_errors_return_500() {
        let all_errors = ApiError::all();
        for error in &all_errors {
            assert_eq!(
                error.status_code(),
                StatusCode::INTERNAL_SERVER_ERROR,
                "All Rosetta errors must be 500"
            );
        }
    }

    #[test]
    fn test_retriable_errors() {
        // These specific errors should be retriable
        assert!(ApiError::AccountNotFound(None).retriable());
        assert!(ApiError::BlockNotFound(None).retriable());
        assert!(ApiError::MempoolIsFull(None).retriable());
        assert!(ApiError::GasEstimationFailed(None).retriable());
        assert!(ApiError::CoinTypeFailedToBeFetched(None).retriable());

        // These should NOT be retriable
        assert!(!ApiError::TransactionIsPending.retriable());
        assert!(!ApiError::NetworkIdentifierMismatch.retriable());
        assert!(!ApiError::ChainIdMismatch.retriable());
        assert!(!ApiError::DeserializationFailed(None).retriable());
        assert!(!ApiError::InvalidSignatureType.retriable());
        assert!(!ApiError::NodeIsOffline.retriable());
        assert!(!ApiError::InternalError(None).retriable());
        assert!(!ApiError::VmError(None).retriable());
        assert!(!ApiError::VersionPruned(None).retriable());
        assert!(!ApiError::BlockPruned(None).retriable());
        assert!(!ApiError::InvalidInput(None).retriable());
    }

    #[test]
    fn test_specific_error_codes() {
        assert_eq!(ApiError::TransactionIsPending.code(), 1);
        assert_eq!(ApiError::NetworkIdentifierMismatch.code(), 2);
        assert_eq!(ApiError::ChainIdMismatch.code(), 3);
        assert_eq!(ApiError::DeserializationFailed(None).code(), 4);
        assert_eq!(ApiError::InvalidTransferOperations(None).code(), 5);
        assert_eq!(ApiError::NodeIsOffline.code(), 14);
        assert_eq!(ApiError::InternalError(None).code(), 17);
        assert_eq!(ApiError::AccountNotFound(None).code(), 18);
        assert_eq!(ApiError::BlockNotFound(None).code(), 25);
        assert_eq!(ApiError::RejectedByFilter(None).code(), 35);
    }

    #[test]
    fn test_error_details_present() {
        let error = ApiError::DeserializationFailed(Some("bad data".to_string()));
        let details = error.details();
        assert!(details.is_some());
        assert_eq!(details.unwrap().details, "bad data");
    }

    #[test]
    fn test_error_details_none_for_simple_errors() {
        let error = ApiError::TransactionIsPending;
        assert!(error.details().is_none());

        let error = ApiError::InvalidSignatureType;
        assert!(error.details().is_none());

        let error = ApiError::NodeIsOffline;
        assert!(error.details().is_none());
    }

    #[test]
    fn test_error_details_various_types() {
        let error = ApiError::InvalidTransferOperations(Some("bad ops"));
        let details = error.details();
        assert_eq!(details.unwrap().details, "bad ops");

        let error = ApiError::UnsupportedSignatureCount(Some(3));
        let details = error.details();
        assert_eq!(details.unwrap().details, "3");

        let error = ApiError::MaxGasFeeTooLow(Some("fee too low".to_string()));
        let details = error.details();
        assert_eq!(details.unwrap().details, "fee too low");
    }

    #[test]
    fn test_into_error_conversion() {
        let api_error = ApiError::InternalError(Some("test error".to_string()));
        let code = api_error.code();
        let message = api_error.message().to_string();
        let retriable = api_error.retriable();
        let error: types::Error = api_error.into();

        assert_eq!(error.code, code);
        assert_eq!(error.message, message);
        assert_eq!(error.retriable, retriable);
        assert!(error.details.is_some());
        assert_eq!(error.details.unwrap().details, "test error");
    }

    #[test]
    fn test_into_error_no_details() {
        let api_error = ApiError::NodeIsOffline;
        let error: types::Error = api_error.into();
        assert_eq!(error.code, 14);
        assert!(!error.retriable);
        assert!(error.details.is_none());
    }

    #[test]
    fn test_deserialization_failed_helper() {
        let error = ApiError::deserialization_failed("MyType");
        assert_eq!(error.code(), 4);
        match error {
            ApiError::DeserializationFailed(Some(msg)) => assert_eq!(msg, "MyType"),
            _ => panic!("Expected DeserializationFailed"),
        }
    }

    #[test]
    fn test_from_account_address_parse_error() {
        let err = AccountAddressParseError::LeadingZeroXRequired;
        let api_error: ApiError = err.into();
        assert_eq!(api_error.code(), 4); // DeserializationFailed
    }

    #[test]
    fn test_from_hex_error() {
        let err = FromHexError::OddLength;
        let api_error: ApiError = err.into();
        assert_eq!(api_error.code(), 4); // DeserializationFailed
    }

    #[test]
    fn test_from_bcs_error() {
        let err = bcs::Error::Eof;
        let api_error: ApiError = err.into();
        assert_eq!(api_error.code(), 4); // DeserializationFailed
    }

    #[test]
    fn test_from_anyhow_error() {
        let err = anyhow::anyhow!("something went wrong");
        let api_error: ApiError = err.into();
        assert_eq!(api_error.code(), 17); // InternalError
    }

    #[test]
    fn test_from_parse_int_error() {
        let err = "not_a_number".parse::<u64>().unwrap_err();
        let api_error: ApiError = err.into();
        assert_eq!(api_error.code(), 4); // DeserializationFailed
    }

    #[test]
    fn test_display_impl() {
        let error = ApiError::NodeIsOffline;
        let display = format!("{}", error);
        assert_eq!(display, "NodeIsOffline");

        let error = ApiError::InternalError(Some("details".to_string()));
        let display = format!("{}", error);
        assert!(display.contains("InternalError"));
        assert!(display.contains("details"));
    }

    #[test]
    fn test_error_is_std_error() {
        let error = ApiError::NodeIsOffline;
        let _: &dyn std::error::Error = &error;
    }

    #[test]
    fn test_rest_error_api_mapping() {
        use aptos_rest_client::{
            aptos_api_types::{AptosError, AptosErrorCode},
            error::{AptosErrorResponse, RestError},
        };
        use warp::http::StatusCode as WarpStatusCode;

        let rest_error = RestError::Api(AptosErrorResponse {
            error: AptosError {
                message: "account gone".to_string(),
                error_code: AptosErrorCode::AccountNotFound,
                vm_error_code: None,
            },
            status_code: WarpStatusCode::NOT_FOUND,
            state: None,
        });
        let api_error: ApiError = rest_error.into();
        assert_eq!(api_error.code(), 18); // AccountNotFound
        assert!(api_error.retriable());

        let rest_error = RestError::Api(AptosErrorResponse {
            error: AptosError {
                message: "vm fail".to_string(),
                error_code: AptosErrorCode::VmError,
                vm_error_code: None,
            },
            status_code: WarpStatusCode::BAD_REQUEST,
            state: None,
        });
        let api_error: ApiError = rest_error.into();
        assert_eq!(api_error.code(), 31); // VmError
    }

    #[test]
    fn test_rest_error_non_api_mapping() {
        use aptos_rest_client::error::RestError;

        let rest_error = RestError::Unknown(anyhow::anyhow!("unknown"));
        let api_error: ApiError = rest_error.into();
        assert_eq!(api_error.code(), 17); // InternalError
    }
}
