// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};
use warp::{http::StatusCode, reject::Reject};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ServiceError {
    pub code: u16,
    pub message: String,
}

impl ServiceError {
    pub fn new(code: StatusCode, message: String) -> Self {
        Self {
            code: code.as_u16(),
            message,
        }
    }

    pub fn from_anyhow_error(code: StatusCode, err: anyhow::Error) -> Self {
        Self::new(code, err.to_string())
    }

    pub fn bad_request<S: Display>(msg: S) -> Self {
        Self::new(StatusCode::BAD_REQUEST, msg.to_string())
    }

    pub fn invalid_request_body<S: Display>(msg: S) -> Self {
        Self::bad_request(format!("invalid request body: {}", msg))
    }

    pub fn unauthorized<S: Display>(msg: S) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, msg.to_string())
    }

    pub fn forbidden<S: Display>(msg: S) -> Self {
        Self::new(StatusCode::FORBIDDEN, msg.to_string())
    }

    pub fn internal(err: anyhow::Error) -> Self {
        Self::from_anyhow_error(StatusCode::INTERNAL_SERVER_ERROR, err)
    }

    pub fn status_code(&self) -> StatusCode {
        StatusCode::from_u16(self.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.status_code(), &self.message)?;
        Ok(())
    }
}

impl Reject for ServiceError {}

impl From<anyhow::Error> for ServiceError {
    fn from(e: anyhow::Error) -> Self {
        Self::internal(e)
    }
}

impl From<serde_json::error::Error> for ServiceError {
    fn from(err: serde_json::error::Error) -> Self {
        Self::internal(err.into())
    }
}
