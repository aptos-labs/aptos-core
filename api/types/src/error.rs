// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt;
use warp::{http::StatusCode, reject::Reject};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct Error {
    pub code: u16,
    pub message: String,
}

impl Error {
    pub fn new(code: StatusCode, message: String) -> Self {
        Self {
            code: code.as_u16(),
            message,
        }
    }

    pub fn from_anyhow_error(code: StatusCode, err: anyhow::Error) -> Self {
        Self::new(code, err.to_string())
    }

    pub fn bad_request(err: anyhow::Error) -> Self {
        Self::from_anyhow_error(StatusCode::BAD_REQUEST, err)
    }

    pub fn not_found(message: String) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
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
        write!(f, "{}: {}", self.status_code(), &self.message)
    }
}

impl Reject for Error {}

impl From<anyhow::Error> for Error {
    fn from(e: anyhow::Error) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
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
    fn test_from_anyhow_error() {
        let err = Error::from(anyhow::format_err!("hello"));
        assert_eq!(err.to_string(), "500 Internal Server Error: hello")
    }
}
