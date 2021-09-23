// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use move_binary_format::errors::PartialVMError;

use serde::{Deserialize, Serialize};
use std::{convert::From, fmt};
use warp::{http::StatusCode, reject::Reject};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Error {
    pub code: u16,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl Error {
    pub fn new(code: StatusCode, message: String) -> Self {
        Self {
            code: code.as_u16(),
            message,
            data: None,
        }
    }

    pub fn new_with_data(code: StatusCode, message: String, data: serde_json::Value) -> Self {
        Self {
            code: code.as_u16(),
            message,
            data: Some(data),
        }
    }

    pub fn from_anyhow_error(code: StatusCode, err: anyhow::Error) -> Self {
        Self::new(code, err.to_string())
    }

    pub fn bad_request(err: anyhow::Error) -> Self {
        Self::from_anyhow_error(StatusCode::BAD_REQUEST, err)
    }

    pub fn not_found(message: String, data: serde_json::Value) -> Self {
        Self::new_with_data(StatusCode::NOT_FOUND, message, data)
    }

    pub fn internal(err: anyhow::Error) -> Self {
        Self::from_anyhow_error(StatusCode::INTERNAL_SERVER_ERROR, err)
    }

    pub fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.status_code(), &self.message)?;
        if let Some(val) = &self.data {
            write!(f, "\n{}", val)?;
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

impl From<PartialVMError> for Error {
    fn from(err: PartialVMError) -> Self {
        Self::internal(anyhow::format_err!(err.to_string()))
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
    fn test_to_string_with_data() {
        let err = Error::new_with_data(
            StatusCode::BAD_REQUEST,
            "invalid address".to_owned(),
            serde_json::json!({"hello": "world"}),
        );
        assert_eq!(
            err.to_string(),
            "400 Bad Request: invalid address\n{\"hello\":\"world\"}"
        )
    }

    #[test]
    fn test_internal_error() {
        let err = Error::internal(anyhow::format_err!("hello"));
        assert_eq!(err.to_string(), "500 Internal Server Error: hello")
    }
}
