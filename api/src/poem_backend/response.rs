// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::fmt::Display;

use super::accept_type::AcceptType;
use anyhow::format_err;
use aptos_api_types::{LedgerInfo, U64};
use poem::Result as PoemResult;
use poem_openapi::{payload::Json, types::ToJSON, ApiResponse, Enum, Object, ResponseContent};
use serde::{Deserialize, Serialize};

use super::bcs_payload::Bcs;

// This should be used for endpoints, signalling that they return either a
// response capturing success or failure.
pub type AptosResponseResult<T> = PoemResult<AptosResponse<T>, AptosErrorResponse>;

// This should be used for internal functions that need to return just a T
// but could fail, in which case we bubble an error response up to the client.
pub type AptosInternalResult<T> = anyhow::Result<T, AptosErrorResponse>;

// TODO: Consdider having more specific error structs for different endpoints.
/// This is the generic struct we use for all API errors, it contains a string
/// message and an Aptos API specific error code.
#[derive(Debug, Object)]
pub struct AptosError {
    message: String,
    error_code: Option<AptosErrorCode>,
    aptos_ledger_version: Option<U64>,
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

/// These codes provide more granular error information beyond just the HTTP
/// status code of the response.
// Make sure the integer codes increment one by one.
#[derive(Debug, Enum)]
pub enum AptosErrorCode {
    /// The Accept header contained an unsupported Accept type.
    UnsupportedAcceptType = 0,

    /// The API failed to read from storage for this request, not because of a
    /// bad request, but because of some internal error.
    ReadFromStorageError = 1,

    /// The data we read from the DB was not valid BCS.
    InvalidBcsInStorageError = 2,

    /// We were unexpectedly unable to convert a Rust type to BCS.
    BcsSerializationError = 3,
}

// TODO: Find a less repetitive way to do this.
#[derive(ApiResponse)]
pub enum AptosErrorResponse {
    #[oai(status = 400)]
    BadRequest(Json<AptosError>),

    #[oai(status = 404)]
    NotFound(Json<AptosError>),

    #[oai(status = 500)]
    InternalServerError(Json<AptosError>),
}

#[derive(ResponseContent)]
pub enum AptosResponseContent<T: ToJSON + Send + Sync> {
    // When returning data as JSON, we take in T and then serialize to JSON
    // as part of the response.
    Json(Json<T>),

    // When returning data as BCS, we never actually interact with the Rust
    // type. Instead, we just return the bytes we read from the DB directly,
    // for efficiency reasons. Only through the `schema` decalaration at the
    // endpoints does the return type make its way into the OpenAPI spec.
    #[oai(actual_type = "Bcs<T>")]
    Bcs(Bcs<Vec<u8>>),
}

#[derive(ApiResponse)]
pub enum AptosResponse<T: ToJSON + Send + Sync> {
    #[oai(status = 200)]
    Ok(
        AptosResponseContent<T>,
        #[oai(header = "X-Aptos-Chain-Id")] u16,
        #[oai(header = "X-Aptos-Ledger-Version")] u64,
        #[oai(header = "X-Aptos-Ledger-Oldest-Version")] u64,
        #[oai(header = "X-Aptos-Ledger-TimestampUsec")] u64,
        #[oai(header = "X-Aptos-Epoch")] u64,
    ),
}

// From impls

impl From<anyhow::Error> for AptosError {
    fn from(error: anyhow::Error) -> Self {
        AptosError::new(error.to_string())
    }
}

impl AptosErrorResponse {
    pub fn not_found<S: Display>(resource: &str, identifier: S, ledger_version: u64) -> Self {
        Self::NotFound(Json(
            AptosError::new(format!("{} not found by {}", resource, identifier))
                .aptos_ledger_version(ledger_version),
        ))
    }

    pub fn invalid_param<S: Display>(name: &str, value: S) -> Self {
        Self::BadRequest(Json(AptosError::new(format!(
            "invalid parameter {}: {}",
            name, value
        ))))
    }
}

impl<T: ToJSON + Send + Sync + Serialize> AptosResponse<T> {
    fn from_ledger_info(content: AptosResponseContent<T>, ledger_info: &LedgerInfo) -> Self {
        AptosResponse::Ok(
            content,
            ledger_info.chain_id as u16,
            ledger_info.ledger_version.into(),
            ledger_info.oldest_ledger_version.into(),
            ledger_info.ledger_timestamp.into(),
            ledger_info.epoch,
        )
    }

    /// Construct a response from bytes that you know ahead of time a BCS
    /// encoded value.
    pub fn from_bcs(value: Vec<u8>, ledger_info: &LedgerInfo) -> Self {
        Self::from_ledger_info(AptosResponseContent::Bcs(Bcs(value)), ledger_info)
    }

    /// Construct an Aptos response from a Rust type, serializing it to JSON.
    pub fn from_json(value: T, ledger_info: &LedgerInfo) -> Self {
        Self::from_ledger_info(AptosResponseContent::Json(Json(value)), ledger_info)
    }

    /// This is a convenience function for creating a response when you have
    /// a Rust object from the beginning. If you're starting out with bytes,
    /// you should instead check the accept type and use either `from_bcs`
    /// or `from_json`.
    pub fn try_from_rust_value(
        value: T,
        ledger_info: &LedgerInfo,
        accept_type: &AcceptType,
    ) -> Result<Self, AptosErrorResponse> {
        match accept_type {
            AcceptType::Bcs => Ok(AptosResponse::from_bcs(
                serialize_to_bcs(&value)?,
                ledger_info,
            )),
            AcceptType::Json => Ok(AptosResponse::from_json(value, ledger_info)),
        }
    }
}

/// Serialize an internal Rust type to BCS, returning a 500 if it fails.
pub fn serialize_to_bcs<T: Serialize>(value: T) -> Result<Vec<u8>, AptosErrorResponse> {
    bcs::to_bytes(&value).map_err(|e| {
        AptosErrorResponse::InternalServerError(Json(
            AptosError::new(
                format_err!("Rust type could not be serialized to BCS: {}", e).to_string(),
            )
            .error_code(AptosErrorCode::BcsSerializationError),
        ))
    })
}

/// Deserialize BCS bytes into an internal Rust, returning a 500 if it fails.
pub fn deserialize_from_bcs<T: for<'b> Deserialize<'b>>(
    bytes: &[u8],
) -> Result<T, AptosErrorResponse> {
    bcs::from_bytes(bytes).map_err(|e| {
        AptosErrorResponse::InternalServerError(Json(
            AptosError::new(format_err!("Data in storage was not valid BCS: {}", e).to_string())
                .error_code(AptosErrorCode::InvalidBcsInStorageError),
        ))
    })
}
