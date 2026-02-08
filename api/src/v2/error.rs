// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Structured error types for the v2 API.
//!
//! V2Error is the single error type returned by all v2 endpoints. It implements
//! `axum::response::IntoResponse` so it can be used directly as a handler return type.
//! Errors do NOT include ledger metadata.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The standard error response for all v2 API endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct V2Error {
    /// Machine-readable error code.
    pub code: ErrorCode,
    /// Human-readable error description.
    pub message: String,
    /// Request ID for log correlation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// Additional structured details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    /// VM status code, if the error originated from Move VM execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vm_status_code: Option<u64>,
    /// HTTP status code (not serialized into JSON body).
    #[serde(skip)]
    http_status: StatusCode,
}

/// Machine-readable error codes for the v2 API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // General
    InternalError,
    InvalidInput,
    NotFound,
    Gone,
    Forbidden,
    ServiceUnavailable,
    RateLimited,
    PayloadTooLarge,

    // BCS
    InvalidBcsVersion,
    InvalidBcsPayload,

    // Resource/state
    AccountNotFound,
    ResourceNotFound,
    ModuleNotFound,
    TableItemNotFound,
    StateValueNotFound,

    // Version/block
    VersionNotFound,
    VersionPruned,
    BlockNotFound,
    BlockPruned,

    // Transaction
    TransactionNotFound,
    MempoolRejected,
    MempoolFull,
    SimulationFailed,

    // View function
    ViewFunctionFailed,
    ViewFunctionForbidden,

    // Batch
    BatchTooLarge,
    BatchRequestFailed,
    MethodNotFound,

    // WebSocket
    WebSocketDisabled,
    WebSocketConnectionLimitReached,
    WebSocketSubscriptionLimitReached,

    // Gas
    GasEstimationFailed,
}

impl ErrorCode {
    /// Map each error code to its HTTP status code.
    pub fn http_status(&self) -> StatusCode {
        match self {
            ErrorCode::InvalidInput
            | ErrorCode::InvalidBcsVersion
            | ErrorCode::InvalidBcsPayload
            | ErrorCode::ViewFunctionFailed
            | ErrorCode::SimulationFailed
            | ErrorCode::BatchTooLarge
            | ErrorCode::MethodNotFound => StatusCode::BAD_REQUEST,

            ErrorCode::Forbidden | ErrorCode::ViewFunctionForbidden => StatusCode::FORBIDDEN,

            ErrorCode::NotFound
            | ErrorCode::AccountNotFound
            | ErrorCode::ResourceNotFound
            | ErrorCode::ModuleNotFound
            | ErrorCode::TableItemNotFound
            | ErrorCode::StateValueNotFound
            | ErrorCode::VersionNotFound
            | ErrorCode::BlockNotFound
            | ErrorCode::TransactionNotFound => StatusCode::NOT_FOUND,

            ErrorCode::Gone | ErrorCode::VersionPruned | ErrorCode::BlockPruned => {
                StatusCode::GONE
            },

            ErrorCode::PayloadTooLarge => StatusCode::PAYLOAD_TOO_LARGE,

            ErrorCode::MempoolRejected => StatusCode::UNPROCESSABLE_ENTITY,

            ErrorCode::RateLimited
            | ErrorCode::WebSocketConnectionLimitReached
            | ErrorCode::WebSocketSubscriptionLimitReached => StatusCode::TOO_MANY_REQUESTS,

            ErrorCode::InternalError
            | ErrorCode::GasEstimationFailed
            | ErrorCode::BatchRequestFailed => StatusCode::INTERNAL_SERVER_ERROR,

            ErrorCode::ServiceUnavailable | ErrorCode::MempoolFull => {
                StatusCode::SERVICE_UNAVAILABLE
            },

            ErrorCode::WebSocketDisabled => StatusCode::NOT_IMPLEMENTED,
        }
    }
}

impl V2Error {
    /// Create an internal server error.
    pub fn internal<E: std::fmt::Display>(err: E) -> Self {
        Self {
            code: ErrorCode::InternalError,
            message: err.to_string(),
            request_id: None,
            details: None,
            vm_status_code: None,
            http_status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Create a bad request error.
    pub fn bad_request<M: Into<String>>(code: ErrorCode, message: M) -> Self {
        Self {
            http_status: code.http_status(),
            code,
            message: message.into(),
            request_id: None,
            details: None,
            vm_status_code: None,
        }
    }

    /// Create a not-found error.
    pub fn not_found<M: Into<String>>(code: ErrorCode, message: M) -> Self {
        Self {
            http_status: code.http_status(),
            code,
            message: message.into(),
            request_id: None,
            details: None,
            vm_status_code: None,
        }
    }

    /// Create a gone (pruned) error.
    pub fn gone<M: Into<String>>(code: ErrorCode, message: M) -> Self {
        Self {
            http_status: code.http_status(),
            code,
            message: message.into(),
            request_id: None,
            details: None,
            vm_status_code: None,
        }
    }

    /// Create a forbidden error.
    #[allow(dead_code)]
    pub fn forbidden<M: Into<String>>(code: ErrorCode, message: M) -> Self {
        Self {
            http_status: code.http_status(),
            code,
            message: message.into(),
            request_id: None,
            details: None,
            vm_status_code: None,
        }
    }

    /// Attach a request ID.
    #[allow(dead_code)]
    pub fn with_request_id(mut self, id: String) -> Self {
        self.request_id = Some(id);
        self
    }

    /// Attach additional details.
    #[allow(dead_code)]
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    /// Get the HTTP status code.
    pub fn http_status(&self) -> StatusCode {
        self.http_status
    }
}

impl IntoResponse for V2Error {
    fn into_response(self) -> Response {
        let status = self.http_status;
        let body = Json(self);
        (status, body).into_response()
    }
}

impl From<anyhow::Error> for V2Error {
    fn from(err: anyhow::Error) -> Self {
        V2Error::internal(err)
    }
}

impl std::fmt::Display for V2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl std::error::Error for V2Error {}
