// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};
use std::{
    convert::From,
    fmt::{self, Display},
};
use warp::{http::StatusCode, reject::Reject};

use crate::U64;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Error {
    pub code: u16,
    pub message: String,
    /// Aptos blockchain latest onchain ledger version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aptos_ledger_version: Option<U64>,
}

impl Error {
    pub fn new(code: StatusCode, message: String) -> Self {
        Self {
            code: code.as_u16(),
            message,
            aptos_ledger_version: None,
        }
    }

    pub fn from_anyhow_error(code: StatusCode, err: anyhow::Error) -> Self {
        Self::new(code, err.to_string())
    }

    pub fn bad_request<S: Display>(msg: S) -> Self {
        Self::new(StatusCode::BAD_REQUEST, msg.to_string())
    }

    pub fn not_found<S: Display>(resource: &str, identifier: S, ledger_version: u64) -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            format!("{} not found by {}", resource, identifier),
        )
        .aptos_ledger_version(ledger_version)
    }

    pub fn invalid_param<S: Display>(name: &str, value: S) -> Self {
        Self::bad_request(format!("invalid parameter {}: {}", name, value))
    }

    pub fn invalid_request_body<S: Display>(msg: S) -> Self {
        Self::bad_request(format!("invalid request body: {}", msg))
    }

    pub fn insufficient_storage<S: Display>(msg: S) -> Self {
        Self::new(StatusCode::INSUFFICIENT_STORAGE, msg.to_string())
    }

    pub fn internal(err: anyhow::Error) -> Self {
        Self::from_anyhow_error(StatusCode::INTERNAL_SERVER_ERROR, err)
    }

    pub fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    pub fn aptos_ledger_version(mut self, ledger_version: u64) -> Self {
        self.aptos_ledger_version = Some(ledger_version.into());
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.status_code(), &self.message)?;
        if let Some(val) = &self.aptos_ledger_version {
            write!(f, "\nAptos ledger version: {}", val)?;
        }
        Ok(())
    }
}

impl Reject for Error {}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::internal(e)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(err: serde_json::error::Error) -> Self {
        Self::internal(err.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::error::Error;
    use warp::http::StatusCode;

    #[test]
    fn test_to_string() {
        let err = Error::new(StatusCode::BAD_REQUEST, "invalid address".to_owned());
        assert_eq!(err.to_string(), "400 Bad Request: invalid address")
    }

    #[test]
    fn test_from_anyhow_error_as_internal_error() {
        let err = Error::from(anyhow::format_err!("hello"));
        assert_eq!(err.to_string(), "500 Internal Server Error: hello")
    }

    #[test]
    fn test_to_string_with_aptos_ledger_version() {
        let err = Error::new(StatusCode::BAD_REQUEST, "invalid address".to_owned())
            .aptos_ledger_version(123);
        assert_eq!(
            err.to_string(),
            "400 Bad Request: invalid address\nAptos ledger version: 123"
        )
    }

    #[test]
    fn test_internal_error() {
        let err = Error::internal(anyhow::format_err!("hello"));
        assert_eq!(err.to_string(), "500 Internal Server Error: hello")
    }
}

// Above is v0 (to be deleted soon), below is v1.

/// This is the generic struct we use for all API errors, it contains a string
/// message and an Aptos API specific error code.
#[derive(Debug, Deserialize, Object)]
pub struct AptosError {
    pub message: String,
    pub error_code: Option<AptosErrorCode>,
    pub aptos_ledger_version: Option<U64>,
}

impl AptosError {
    pub fn new(message: String) -> Self {
        Self {
            message,
            error_code: None,
            aptos_ledger_version: None,
        }
    }

    pub fn error_code(mut self, error_code: AptosErrorCode) -> Self {
        self.error_code = Some(error_code);
        self
    }

    pub fn aptos_ledger_version(mut self, ledger_version: u64) -> Self {
        self.aptos_ledger_version = Some(ledger_version.into());
        self
    }
}

impl From<anyhow::Error> for AptosError {
    fn from(error: anyhow::Error) -> Self {
        AptosError::new(format!("{:#}", error))
    }
}

/// These codes provide more granular error information beyond just the HTTP
/// status code of the response.
// Make sure the integer codes increment one by one.
#[derive(Debug, Deserialize, Enum)]
#[oai(rename_all = "snake_case")]
pub enum AptosErrorCode {
    /// The API failed to read from storage for this request, not because of a
    /// bad request, but because of some internal error.
    ReadFromStorageError = 1,

    /// The data we read from the DB was not valid BCS.
    InvalidBcsInStorageError = 2,

    /// We were unexpectedly unable to convert a Rust type to BCS.
    BcsSerializationError = 3,

    /// The start param given for paging is invalid.
    InvalidStartParam = 4,

    /// The limit param given for paging is invalid.
    InvalidLimitParam = 5,
}
