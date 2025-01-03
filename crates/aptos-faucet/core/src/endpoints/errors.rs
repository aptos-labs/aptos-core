// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::middleware::bump_rejection_reason_counters;
use once_cell::sync::OnceCell;
use poem::http::StatusCode;
use poem_openapi::{payload::Json, ApiResponse, Enum, Object};
use std::fmt::Formatter;

pub static USE_HELPFUL_ERRORS: OnceCell<bool> = OnceCell::new();

/// This is the generic struct we use for all API errors, it contains a string
/// message and a service specific error code.
#[derive(Debug, Clone, Object)]
pub struct AptosTapError {
    /// A message describing the error
    pub message: String,
    /// A code describing the error for programmatic use.
    pub error_code: AptosTapErrorCode,
    /// If we're returning a 403 because we're rejecting the mint request, this
    /// contains additional reasons why.
    pub rejection_reasons: Vec<RejectionReason>,
    /// Submitted transaction hashes, if it got to that point.
    pub txn_hashes: Vec<String>,
}

impl AptosTapError {
    pub fn new(message: String, error_code: AptosTapErrorCode) -> Self {
        if *USE_HELPFUL_ERRORS.get().unwrap_or(&true) {
            Self {
                message,
                error_code,
                rejection_reasons: vec![],
                txn_hashes: vec![],
            }
        } else {
            Self {
                message: "hah hah hah".to_string(),
                error_code: AptosTapErrorCode::YeahNahYeahYeahYeahNahYeahNah,
                rejection_reasons: vec![],
                txn_hashes: vec![],
            }
        }
    }

    pub fn new_with_error_code<ErrorType: std::fmt::Display>(
        error: ErrorType,
        error_code: AptosTapErrorCode,
    ) -> AptosTapError {
        Self::new(format!("{:#}", error), error_code)
    }

    pub fn rejection_reasons(mut self, rejection_reasons: Vec<RejectionReason>) -> Self {
        self.rejection_reasons = rejection_reasons;
        self
    }

    pub fn txn_hashes(mut self, txn_hashes: Vec<String>) -> Self {
        self.txn_hashes = txn_hashes;
        self
    }

    // If we're telling people to try again later, it's a 429.
    pub fn status_and_retry_after(&self) -> (StatusCode, Option<u64>) {
        let (mut status_code, mut retry_after) = (self.error_code.status(), None);
        for rejection_reason in &self.rejection_reasons {
            if rejection_reason.code == RejectionReasonCode::UsageLimitExhausted {
                status_code = StatusCode::TOO_MANY_REQUESTS;
                retry_after = rejection_reason.retry_after;
                break;
            }
        }
        (status_code, retry_after)
    }
}

impl std::fmt::Display for AptosTapError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Error({}): {}: {:?}",
            self.error_code as u32, self.message, self.rejection_reasons
        )
    }
}

impl std::error::Error for AptosTapError {}

// This is only really necessary for the type of the error in the Result
// returned by the endpoint handlers.
#[derive(Debug, ApiResponse)]
pub enum AptosTapErrorResponse {
    Default(
        StatusCode,
        Json<AptosTapError>,
        #[oai(header = "Retry-After")] Option<u64>,
    ),
}

impl From<AptosTapError> for AptosTapErrorResponse {
    fn from(error: AptosTapError) -> Self {
        // We use this opportunity to bump metrics based on the specifics of
        // this response, since this function is only called right when we're
        // about to return this error to the client.
        bump_rejection_reason_counters(&error.rejection_reasons);
        let (status, retry_after) = error.status_and_retry_after();
        Self::Default(status, Json(error), retry_after)
    }
}

/// These codes provide more granular error information beyond just the HTTP
/// status code of the response.
#[derive(Copy, Clone, Debug, Enum, Eq, PartialEq)]
#[repr(u32)]
pub enum AptosTapErrorCode {
    /// Intentionally unhelpful error code.
    YeahNahYeahYeahYeahNahYeahNah = 1,

    /// The request itself was invalid.
    InvalidRequest = 40,

    /// The account to fund in the request did not exist.
    AccountDoesNotExist = 41,

    /// Client was rejected. See `rejection_reasons` for more details.
    Rejected = 42,

    /// There was no source IP in the request.
    SourceIpMissing = 43,

    /// Transaction failed, likely because a precondition was violated.
    TransactionFailed = 44,

    /// The user tried to call an endpoint that is not enabled.
    EndpointNotEnabled = 45,

    /// The user provided an invalid auth token.
    AuthTokenInvalid = 46,

    /// Failed when making requests to the Aptos API.
    AptosApiError = 50,

    /// One of the Bypassers failed.
    BypasserError = 51,

    /// One of the validity checkers failed.
    CheckerError = 52,

    /// Failed to read to / write from storage.
    StorageError = 53,

    /// Something is wrong with the funder account.
    FunderAccountProblem = 54,

    /// Transaction to fund account timed out.
    TransactionTimedOut = 55,

    /// Failed to serialize response.
    SerializationError = 56,

    /// The server has hit its max concurrent requests limit.
    ServerOverloaded = 57,

    /// Error from the web framework.
    WebFrameworkError = 60,
}

impl AptosTapErrorCode {
    pub fn status(&self) -> StatusCode {
        match self {
            AptosTapErrorCode::InvalidRequest
            | AptosTapErrorCode::AccountDoesNotExist
            | AptosTapErrorCode::EndpointNotEnabled => StatusCode::BAD_REQUEST,
            AptosTapErrorCode::Rejected
            | AptosTapErrorCode::SourceIpMissing
            | AptosTapErrorCode::TransactionFailed
            | AptosTapErrorCode::AuthTokenInvalid => StatusCode::FORBIDDEN,
            AptosTapErrorCode::AptosApiError
            | AptosTapErrorCode::TransactionTimedOut
            | AptosTapErrorCode::SerializationError
            | AptosTapErrorCode::BypasserError
            | AptosTapErrorCode::CheckerError
            | AptosTapErrorCode::StorageError => StatusCode::INTERNAL_SERVER_ERROR,
            AptosTapErrorCode::ServerOverloaded | AptosTapErrorCode::FunderAccountProblem => {
                StatusCode::SERVICE_UNAVAILABLE
            },
            AptosTapErrorCode::YeahNahYeahYeahYeahNahYeahNah => StatusCode::IM_A_TEAPOT,
            // We shouldn't get here, this code is only used in error_converter.rs.
            AptosTapErrorCode::WebFrameworkError => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Clone, Object)]
pub struct RejectionReason {
    reason: String,
    code: RejectionReasonCode,
    #[oai(skip)]
    pub retry_after: Option<u64>,
}

impl RejectionReason {
    pub fn new(reason: String, code: RejectionReasonCode) -> Self {
        if *USE_HELPFUL_ERRORS.get().unwrap() {
            Self {
                reason,
                code,
                retry_after: None,
            }
        } else {
            Self {
                reason: "keep dreaming mate".to_string(),
                code: RejectionReasonCode::Hehe,
                retry_after: None,
            }
        }
    }

    pub fn retry_after(mut self, retry_after: u64) -> Self {
        self.retry_after = Some(retry_after);
        self
    }

    pub fn get_code(&self) -> RejectionReasonCode {
        self.code
    }
}

// todo explain that the frontend may not want to display specifics here.
// say this is only for the filters. maybe rename to say filters.
#[derive(Copy, Clone, Debug, Enum, Eq, Hash, PartialEq)]
#[repr(u32)]
pub enum RejectionReasonCode {
    /// Intentionally unhelpful reason code.
    Hehe = 1,

    /// Account already has funds.
    AccountAlreadyExists = 100,

    /// Key (IP / Firebase UID) has exhausted its usage limit.
    UsageLimitExhausted = 101,

    /// IP is in the blocklist.
    IpInBlocklist = 102,

    /// The origin of the request is from a VPN.
    RequestFromVpn = 103,

    /// The origin of the request is a cloud.
    RequestFromCloud = 104,

    /// The request did not contain the required magic header.
    MagicHeaderIncorrect = 105,

    /// The captcha was missing or incorrect.
    CaptchaInvalid = 106,

    /// Auth token was not given, is invalid, or is not allowed by the server.
    AuthTokenInvalid = 107,

    /// Referer was in the blocklist.
    RefererBlocklisted = 108,
}
