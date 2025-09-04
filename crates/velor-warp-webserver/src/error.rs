// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_api_types::U64;
use serde::{Deserialize, Serialize};
use std::{
    convert::From,
    fmt::{self, Display},
};
use warp::{http::StatusCode, reject::Reject};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Error {
    pub code: u16,
    pub message: String,
    /// Velor blockchain latest onchain ledger version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub velor_ledger_version: Option<U64>,
}

impl Error {
    pub fn new(code: StatusCode, message: String) -> Self {
        Self {
            code: code.as_u16(),
            message,
            velor_ledger_version: None,
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
        .velor_ledger_version(ledger_version)
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

    pub fn velor_ledger_version(mut self, ledger_version: u64) -> Self {
        self.velor_ledger_version = Some(ledger_version.into());
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.status_code(), &self.message)?;
        if let Some(val) = &self.velor_ledger_version {
            write!(f, "\nVelor ledger version: {}", val)?;
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
    fn test_to_string_with_velor_ledger_version() {
        let err = Error::new(StatusCode::BAD_REQUEST, "invalid address".to_owned())
            .velor_ledger_version(123);
        assert_eq!(
            err.to_string(),
            "400 Bad Request: invalid address\nVelor ledger version: 123"
        )
    }

    #[test]
    fn test_internal_error() {
        let err = Error::internal(anyhow::format_err!("hello"));
        assert_eq!(err.to_string(), "500 Internal Server Error: hello")
    }
}
