// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::accept_type::AcceptType;
use aptos_api_types::{AptosError, AptosErrorCode, LedgerInfo};
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Serialize;
use std::fmt::Display;

const X_APTOS_CHAIN_ID: &str = "X-Aptos-Chain-Id";
const X_APTOS_LEDGER_VERSION: &str = "X-Aptos-Ledger-Version";
const X_APTOS_LEDGER_OLDEST_VERSION: &str = "X-Aptos-Ledger-Oldest-Version";
const X_APTOS_LEDGER_TIMESTAMP: &str = "X-Aptos-Ledger-TimestampUsec";
const X_APTOS_EPOCH: &str = "X-Aptos-Epoch";
const X_APTOS_BLOCK_HEIGHT: &str = "X-Aptos-Block-Height";
const X_APTOS_OLDEST_BLOCK_HEIGHT: &str = "X-Aptos-Oldest-Block-Height";
const X_APTOS_GAS_USED: &str = "X-Aptos-Gas-Used";
const X_APTOS_CURSOR: &str = "X-Aptos-Cursor";
const X_APTOS_TXN_ENCRYPTION_KEY: &str = "X-Aptos-Txn-Encryption-Key";

fn build_ledger_headers(ledger_info: &LedgerInfo) -> Vec<(String, String)> {
    let mut headers = vec![
        (
            X_APTOS_CHAIN_ID.to_string(),
            ledger_info.chain_id.to_string(),
        ),
        (
            X_APTOS_LEDGER_VERSION.to_string(),
            u64::from(ledger_info.ledger_version).to_string(),
        ),
        (
            X_APTOS_LEDGER_OLDEST_VERSION.to_string(),
            u64::from(ledger_info.oldest_ledger_version).to_string(),
        ),
        (
            X_APTOS_LEDGER_TIMESTAMP.to_string(),
            u64::from(ledger_info.ledger_timestamp).to_string(),
        ),
        (
            X_APTOS_EPOCH.to_string(),
            u64::from(ledger_info.epoch).to_string(),
        ),
        (
            X_APTOS_BLOCK_HEIGHT.to_string(),
            u64::from(ledger_info.block_height).to_string(),
        ),
        (
            X_APTOS_OLDEST_BLOCK_HEIGHT.to_string(),
            u64::from(ledger_info.oldest_block_height).to_string(),
        ),
    ];
    if let Some(ref key) = ledger_info.txn_encryption_key {
        headers.push((X_APTOS_TXN_ENCRYPTION_KEY.to_string(), key.clone()));
    }
    headers
}

fn build_optional_ledger_headers(ledger_info: Option<&LedgerInfo>) -> Vec<(String, String)> {
    match ledger_info {
        Some(info) => build_ledger_headers(info),
        None => vec![],
    }
}

pub struct AptosResponse<T: Serialize + Send> {
    pub status: StatusCode,
    pub body: AptosResponseBody<T>,
    pub ledger_info: LedgerInfo,
    pub gas_used: Option<u64>,
    pub cursor: Option<String>,
}

pub enum AptosResponseBody<T: Serialize + Send> {
    Json(T),
    Bcs(Vec<u8>),
}

impl<T: Serialize + Send> AptosResponse<T> {
    pub fn new_json(status: StatusCode, body: T, ledger_info: &LedgerInfo) -> Self {
        Self {
            status,
            body: AptosResponseBody::Json(body),
            ledger_info: ledger_info.clone(),
            gas_used: None,
            cursor: None,
        }
    }

    pub fn new_bcs(status: StatusCode, bcs_bytes: Vec<u8>, ledger_info: &LedgerInfo) -> Self {
        Self {
            status,
            body: AptosResponseBody::Bcs(bcs_bytes),
            ledger_info: ledger_info.clone(),
            gas_used: None,
            cursor: None,
        }
    }

    pub fn with_gas_used(mut self, gas_used: Option<u64>) -> Self {
        self.gas_used = gas_used;
        self
    }

    pub fn with_cursor(
        mut self,
        cursor: Option<aptos_types::state_store::state_key::StateKey>,
    ) -> Self {
        self.cursor = cursor.map(|c| aptos_api_types::StateKeyWrapper::from(c).to_string());
        self
    }

    #[allow(clippy::result_large_err)]
    pub fn try_from_rust_value(
        value: T,
        ledger_info: &LedgerInfo,
        accept_type: &AcceptType,
    ) -> Result<Self, AptosErrorResponse>
    where
        T: Serialize,
    {
        match accept_type {
            AcceptType::Json => Ok(Self::new_json(StatusCode::OK, value, ledger_info)),
            AcceptType::Bcs => {
                let bytes = bcs::to_bytes(&value).map_err(|e| {
                    AptosErrorResponse::internal(
                        e,
                        AptosErrorCode::InternalError,
                        Some(ledger_info),
                    )
                })?;
                Ok(Self::new_bcs(StatusCode::OK, bytes, ledger_info))
            },
        }
    }

    #[allow(clippy::result_large_err)]
    pub fn try_from_json(value: T, ledger_info: &LedgerInfo) -> Result<Self, AptosErrorResponse> {
        Ok(Self::new_json(StatusCode::OK, value, ledger_info))
    }

    #[allow(clippy::result_large_err)]
    pub fn try_from_bcs<B: Serialize>(
        value: B,
        ledger_info: &LedgerInfo,
    ) -> Result<Self, AptosErrorResponse> {
        let bytes = bcs::to_bytes(&value).map_err(|e| {
            AptosErrorResponse::internal(e, AptosErrorCode::InternalError, Some(ledger_info))
        })?;
        Ok(Self::new_bcs(StatusCode::OK, bytes, ledger_info))
    }

    #[allow(clippy::result_large_err)]
    pub fn try_from_encoded(
        value: Vec<u8>,
        ledger_info: &LedgerInfo,
    ) -> Result<Self, AptosErrorResponse> {
        Ok(Self::new_bcs(StatusCode::OK, value, ledger_info))
    }
}

impl<T: Serialize + Send> IntoResponse for AptosResponse<T> {
    fn into_response(self) -> Response {
        let mut headers = build_ledger_headers(&self.ledger_info);

        if let Some(gas) = self.gas_used {
            headers.push((X_APTOS_GAS_USED.to_string(), gas.to_string()));
        }
        if let Some(cursor) = self.cursor {
            headers.push((X_APTOS_CURSOR.to_string(), cursor));
        }

        let mut response = match self.body {
            AptosResponseBody::Json(body) => {
                let json = serde_json::to_vec(&body).unwrap_or_default();
                (
                    self.status,
                    [(header::CONTENT_TYPE, "application/json")],
                    json,
                )
                    .into_response()
            },
            AptosResponseBody::Bcs(bytes) => (
                self.status,
                [(header::CONTENT_TYPE, aptos_api_types::mime_types::BCS)],
                bytes,
            )
                .into_response(),
        };

        for (key, value) in headers {
            if let (Ok(name), Ok(val)) = (
                http::header::HeaderName::try_from(key),
                http::header::HeaderValue::try_from(value),
            ) {
                response.headers_mut().insert(name, val);
            }
        }

        response
    }
}

#[derive(Debug)]
pub struct AptosErrorResponse {
    pub status: StatusCode,
    pub error: AptosError,
    pub ledger_info: Option<LedgerInfo>,
}

impl AptosErrorResponse {
    pub fn new(status: StatusCode, error: AptosError, ledger_info: Option<&LedgerInfo>) -> Self {
        Self {
            status,
            error,
            ledger_info: ledger_info.cloned(),
        }
    }

    pub fn bad_request<E: Display>(
        err: E,
        error_code: AptosErrorCode,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            AptosError::new_with_error_code(err, error_code),
            ledger_info,
        )
    }

    pub fn not_found<E: Display>(
        err: E,
        error_code: AptosErrorCode,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        Self::new(
            StatusCode::NOT_FOUND,
            AptosError::new_with_error_code(err, error_code),
            ledger_info,
        )
    }

    pub fn forbidden<E: Display>(
        err: E,
        error_code: AptosErrorCode,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        Self::new(
            StatusCode::FORBIDDEN,
            AptosError::new_with_error_code(err, error_code),
            ledger_info,
        )
    }

    pub fn gone<E: Display>(
        err: E,
        error_code: AptosErrorCode,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        Self::new(
            StatusCode::GONE,
            AptosError::new_with_error_code(err, error_code),
            ledger_info,
        )
    }

    pub fn internal<E: Display>(
        err: E,
        error_code: AptosErrorCode,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            AptosError::new_with_error_code(err, error_code),
            ledger_info,
        )
    }

    pub fn service_unavailable<E: Display>(
        err: E,
        error_code: AptosErrorCode,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        Self::new(
            StatusCode::SERVICE_UNAVAILABLE,
            AptosError::new_with_error_code(err, error_code),
            ledger_info,
        )
    }

    pub fn internal_with_vm_status<E: Display>(
        err: E,
        error_code: AptosErrorCode,
        vm_status: aptos_types::vm_status::StatusCode,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            AptosError::new_with_vm_status(err, error_code, vm_status),
            ledger_info,
        )
    }

    pub fn bad_request_with_vm_status<E: Display>(
        err: E,
        error_code: AptosErrorCode,
        vm_status: aptos_types::vm_status::StatusCode,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        Self::new(
            StatusCode::BAD_REQUEST,
            AptosError::new_with_vm_status(err, error_code, vm_status),
            ledger_info,
        )
    }

    pub fn payload_too_large<E: Display>(
        err: E,
        error_code: AptosErrorCode,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        Self::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            AptosError::new_with_error_code(err, error_code),
            ledger_info,
        )
    }

    pub fn insufficient_storage<E: Display>(
        err: E,
        error_code: AptosErrorCode,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        Self::new(
            StatusCode::INSUFFICIENT_STORAGE,
            AptosError::new_with_error_code(err, error_code),
            ledger_info,
        )
    }
}

impl std::fmt::Display for AptosErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {:?}", self.status, self.error)
    }
}

impl std::error::Error for AptosErrorResponse {}

impl IntoResponse for AptosErrorResponse {
    fn into_response(self) -> Response {
        let headers = build_optional_ledger_headers(self.ledger_info.as_ref());

        let json = serde_json::to_vec(&self.error).unwrap_or_default();
        let mut response = (
            self.status,
            [(header::CONTENT_TYPE, "application/json")],
            json,
        )
            .into_response();

        for (key, value) in headers {
            if let (Ok(name), Ok(val)) = (
                http::header::HeaderName::try_from(key),
                http::header::HeaderValue::try_from(value),
            ) {
                response.headers_mut().insert(name, val);
            }
        }

        response
    }
}

pub type AptosResult<T> = Result<AptosResponse<T>, AptosErrorResponse>;

pub fn build_not_found(
    resource: &str,
    identifier: impl Display,
    error_code: AptosErrorCode,
    ledger_info: &LedgerInfo,
) -> AptosErrorResponse {
    AptosErrorResponse::not_found(
        format!("{} not found by {}", resource, identifier),
        error_code,
        Some(ledger_info),
    )
}

pub fn version_not_found(ledger_version: u64, ledger_info: &LedgerInfo) -> AptosErrorResponse {
    build_not_found(
        "Ledger version",
        format!("Ledger version({})", ledger_version),
        AptosErrorCode::VersionNotFound,
        ledger_info,
    )
}

pub fn version_pruned(ledger_version: u64, ledger_info: &LedgerInfo) -> AptosErrorResponse {
    AptosErrorResponse::gone(
        format!("Ledger version({}) has been pruned", ledger_version),
        AptosErrorCode::VersionPruned,
        Some(ledger_info),
    )
}

pub fn account_not_found(
    address: aptos_api_types::Address,
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> AptosErrorResponse {
    build_not_found(
        "Account",
        format!(
            "Address({}) and Ledger version({})",
            address, ledger_version
        ),
        AptosErrorCode::AccountNotFound,
        ledger_info,
    )
}

pub fn resource_not_found(
    address: aptos_api_types::Address,
    struct_tag: &move_core_types::language_storage::StructTag,
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> AptosErrorResponse {
    build_not_found(
        "Resource",
        format!(
            "Address({}), Struct tag({}) and Ledger version({})",
            address,
            struct_tag.to_canonical_string(),
            ledger_version
        ),
        AptosErrorCode::ResourceNotFound,
        ledger_info,
    )
}

pub fn module_not_found(
    address: aptos_api_types::Address,
    module_name: &move_core_types::identifier::IdentStr,
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> AptosErrorResponse {
    build_not_found(
        "Module",
        format!(
            "Address({}), Module name({}) and Ledger version({})",
            address, module_name, ledger_version
        ),
        AptosErrorCode::ModuleNotFound,
        ledger_info,
    )
}

pub fn struct_field_not_found(
    address: aptos_api_types::Address,
    struct_tag: &move_core_types::language_storage::StructTag,
    field_name: &move_core_types::identifier::Identifier,
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> AptosErrorResponse {
    build_not_found(
        "Struct Field",
        format!(
            "Address({}), Struct tag({}), Field name({}) and Ledger version({})",
            address,
            struct_tag.to_canonical_string(),
            field_name,
            ledger_version
        ),
        AptosErrorCode::StructFieldNotFound,
        ledger_info,
    )
}

pub fn table_item_not_found(
    table_handle: aptos_api_types::Address,
    table_key: &serde_json::Value,
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> AptosErrorResponse {
    build_not_found(
        "Table Item",
        format!(
            "Table handle({}), Table key({}) and Ledger version({})",
            table_handle, table_key, ledger_version
        ),
        AptosErrorCode::TableItemNotFound,
        ledger_info,
    )
}

pub fn transaction_not_found_by_version(
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> AptosErrorResponse {
    build_not_found(
        "Transaction",
        format!("Ledger version({})", ledger_version),
        AptosErrorCode::TransactionNotFound,
        ledger_info,
    )
}

pub fn transaction_not_found_by_hash(
    hash: aptos_api_types::HashValue,
    ledger_info: &LedgerInfo,
) -> AptosErrorResponse {
    build_not_found(
        "Transaction",
        format!("Transaction hash({})", hash),
        AptosErrorCode::TransactionNotFound,
        ledger_info,
    )
}

pub fn block_not_found_by_height(
    block_height: u64,
    ledger_info: &LedgerInfo,
) -> AptosErrorResponse {
    build_not_found(
        "Block",
        format!("Block height({})", block_height),
        AptosErrorCode::BlockNotFound,
        ledger_info,
    )
}

pub fn block_not_found_by_version(
    ledger_version: u64,
    ledger_info: &LedgerInfo,
) -> AptosErrorResponse {
    build_not_found(
        "Block",
        format!("Ledger version({})", ledger_version),
        AptosErrorCode::BlockNotFound,
        ledger_info,
    )
}

pub fn block_pruned_by_height(block_height: u64, ledger_info: &LedgerInfo) -> AptosErrorResponse {
    AptosErrorResponse::gone(
        format!("Block({}) has been pruned", block_height),
        AptosErrorCode::BlockPruned,
        Some(ledger_info),
    )
}

pub fn json_api_disabled(identifier: impl Display) -> AptosErrorResponse {
    AptosErrorResponse::forbidden(
        format!(
            "{} with JSON output is disabled on this endpoint",
            identifier
        ),
        AptosErrorCode::ApiDisabled,
        None,
    )
}

pub fn bcs_api_disabled(identifier: impl Display) -> AptosErrorResponse {
    AptosErrorResponse::forbidden(
        format!(
            "{} with BCS output is disabled on this endpoint",
            identifier
        ),
        AptosErrorCode::ApiDisabled,
        None,
    )
}

pub fn api_disabled(identifier: impl Display) -> AptosErrorResponse {
    AptosErrorResponse::forbidden(
        format!("{} is disabled on this endpoint", identifier),
        AptosErrorCode::ApiDisabled,
        None,
    )
}

pub fn api_forbidden(identifier: impl Display, extra_help: impl Display) -> AptosErrorResponse {
    AptosErrorResponse::forbidden(
        format!("{} is not allowed. {}", identifier, extra_help),
        AptosErrorCode::ApiDisabled,
        None,
    )
}

// Implement From conversions from Poem error types to AptosErrorResponse.
// This is the bridge layer that allows Axum handlers to use `?` with existing
// business logic that returns Poem error types.

impl From<crate::response::BasicError> for AptosErrorResponse {
    fn from(err: crate::response::BasicError) -> Self {
        let resp = poem::IntoResponse::into_response(err);
        poem_error_response_to_axum(resp)
    }
}

impl From<crate::response::BasicErrorWith404> for AptosErrorResponse {
    fn from(err: crate::response::BasicErrorWith404) -> Self {
        let resp = poem::IntoResponse::into_response(err);
        poem_error_response_to_axum(resp)
    }
}

impl From<crate::transactions::SubmitTransactionError> for AptosErrorResponse {
    fn from(err: crate::transactions::SubmitTransactionError) -> Self {
        let resp = poem::IntoResponse::into_response(err);
        poem_error_response_to_axum(resp)
    }
}

fn poem_error_response_to_axum(resp: poem::Response) -> AptosErrorResponse {
    let status_code =
        StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    let ledger_info = extract_ledger_info_from_poem_headers(resp.headers());

    // Poem error response bodies are small, in-memory JSON buffers.
    // poem::Body::into_vec() is async, so we must bridge to sync here since
    // From impls cannot be async. block_in_place is safe because this code
    // only runs inside the Axum server's multi-threaded Tokio runtime.
    let body_bytes = match tokio::runtime::Handle::try_current() {
        Ok(handle) => tokio::task::block_in_place(|| {
            handle
                .block_on(resp.into_body().into_vec())
                .unwrap_or_default()
        }),
        Err(_) => Vec::new(),
    };
    let error: AptosError = serde_json::from_slice(&body_bytes).unwrap_or_else(|_| {
        AptosError::new_with_error_code("Unknown error", AptosErrorCode::InternalError)
    });
    AptosErrorResponse {
        status: status_code,
        error,
        ledger_info,
    }
}

fn extract_ledger_info_from_poem_headers(headers: &poem::http::HeaderMap) -> Option<LedgerInfo> {
    fn get_header_u64(headers: &poem::http::HeaderMap, name: &str) -> Option<u64> {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
    }

    fn get_header_u8(headers: &poem::http::HeaderMap, name: &str) -> Option<u8> {
        headers
            .get(name)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse().ok())
    }

    let chain_id = get_header_u8(headers, X_APTOS_CHAIN_ID)?;
    let ledger_version = get_header_u64(headers, X_APTOS_LEDGER_VERSION)?;

    Some(LedgerInfo {
        chain_id,
        ledger_version: ledger_version.into(),
        oldest_ledger_version: get_header_u64(headers, X_APTOS_LEDGER_OLDEST_VERSION)
            .unwrap_or(0)
            .into(),
        ledger_timestamp: get_header_u64(headers, X_APTOS_LEDGER_TIMESTAMP)
            .unwrap_or(0)
            .into(),
        epoch: get_header_u64(headers, X_APTOS_EPOCH).unwrap_or(0).into(),
        block_height: get_header_u64(headers, X_APTOS_BLOCK_HEIGHT)
            .unwrap_or(0)
            .into(),
        oldest_block_height: get_header_u64(headers, X_APTOS_OLDEST_BLOCK_HEIGHT)
            .unwrap_or(0)
            .into(),
        txn_encryption_key: headers
            .get(X_APTOS_TXN_ENCRYPTION_KEY)
            .and_then(|v| v.to_str().ok())
            .map(String::from),
    })
}

// Implement the error traits generated by generate_error_traits! so that Context
// methods (which are generic over these traits) can be called with AptosErrorResponse.
impl crate::response::AptosErrorResponse for AptosErrorResponse {
    fn inner_mut(&mut self) -> &mut AptosError {
        &mut self.error
    }
}

impl crate::response::BadRequestError for AptosErrorResponse {
    fn bad_request_with_code<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::bad_request(err, error_code, Some(ledger_info))
    }

    fn bad_request_with_code_no_info<Err: Display>(err: Err, error_code: AptosErrorCode) -> Self {
        AptosErrorResponse::bad_request(err, error_code, None)
    }

    fn bad_request_with_optional_vm_status_and_ledger_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: Option<aptos_types::vm_status::StatusCode>,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        if let Some(vs) = vm_status {
            AptosErrorResponse::bad_request_with_vm_status(err, error_code, vs, ledger_info)
        } else {
            AptosErrorResponse::bad_request(err, error_code, ledger_info)
        }
    }

    fn bad_request_with_vm_status<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: aptos_types::vm_status::StatusCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::bad_request_with_vm_status(
            err,
            error_code,
            vm_status,
            Some(ledger_info),
        )
    }

    fn bad_request_from_aptos_error(aptos_error: AptosError, ledger_info: &LedgerInfo) -> Self {
        AptosErrorResponse::new(StatusCode::BAD_REQUEST, aptos_error, Some(ledger_info))
    }
}

impl crate::response::GoneError for AptosErrorResponse {
    fn gone_with_code<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::gone(err, error_code, Some(ledger_info))
    }

    fn gone_with_code_no_info<Err: Display>(err: Err, error_code: AptosErrorCode) -> Self {
        AptosErrorResponse::gone(err, error_code, None)
    }

    fn gone_with_optional_vm_status_and_ledger_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: Option<aptos_types::vm_status::StatusCode>,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        if let Some(vs) = vm_status {
            let mut resp = AptosErrorResponse::gone(err, error_code, ledger_info);
            resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vs);
            resp
        } else {
            AptosErrorResponse::gone(err, error_code, ledger_info)
        }
    }

    fn gone_with_vm_status<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: aptos_types::vm_status::StatusCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        let mut resp = AptosErrorResponse::gone(err, error_code, Some(ledger_info));
        resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vm_status);
        resp
    }

    fn gone_from_aptos_error(aptos_error: AptosError, ledger_info: &LedgerInfo) -> Self {
        AptosErrorResponse::new(StatusCode::GONE, aptos_error, Some(ledger_info))
    }
}

impl crate::response::NotFoundError for AptosErrorResponse {
    fn not_found_with_code<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::not_found(err, error_code, Some(ledger_info))
    }

    fn not_found_with_code_no_info<Err: Display>(err: Err, error_code: AptosErrorCode) -> Self {
        AptosErrorResponse::not_found(err, error_code, None)
    }

    fn not_found_with_optional_vm_status_and_ledger_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: Option<aptos_types::vm_status::StatusCode>,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        if let Some(vs) = vm_status {
            let mut resp = AptosErrorResponse::not_found(err, error_code, ledger_info);
            resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vs);
            resp
        } else {
            AptosErrorResponse::not_found(err, error_code, ledger_info)
        }
    }

    fn not_found_with_vm_status<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: aptos_types::vm_status::StatusCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        let mut resp = AptosErrorResponse::not_found(err, error_code, Some(ledger_info));
        resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vm_status);
        resp
    }

    fn not_found_from_aptos_error(aptos_error: AptosError, ledger_info: &LedgerInfo) -> Self {
        AptosErrorResponse::new(StatusCode::NOT_FOUND, aptos_error, Some(ledger_info))
    }
}

impl crate::response::ForbiddenError for AptosErrorResponse {
    fn forbidden_with_code<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::forbidden(err, error_code, Some(ledger_info))
    }

    fn forbidden_with_code_no_info<Err: Display>(err: Err, error_code: AptosErrorCode) -> Self {
        AptosErrorResponse::forbidden(err, error_code, None)
    }

    fn forbidden_with_optional_vm_status_and_ledger_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: Option<aptos_types::vm_status::StatusCode>,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        if let Some(vs) = vm_status {
            let mut resp = AptosErrorResponse::forbidden(err, error_code, ledger_info);
            resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vs);
            resp
        } else {
            AptosErrorResponse::forbidden(err, error_code, ledger_info)
        }
    }

    fn forbidden_with_vm_status<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: aptos_types::vm_status::StatusCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        let mut resp = AptosErrorResponse::forbidden(err, error_code, Some(ledger_info));
        resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vm_status);
        resp
    }

    fn forbidden_from_aptos_error(aptos_error: AptosError, ledger_info: &LedgerInfo) -> Self {
        AptosErrorResponse::new(StatusCode::FORBIDDEN, aptos_error, Some(ledger_info))
    }
}

impl crate::response::InternalError for AptosErrorResponse {
    fn internal_with_code<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::internal(err, error_code, Some(ledger_info))
    }

    fn internal_with_code_no_info<Err: Display>(err: Err, error_code: AptosErrorCode) -> Self {
        AptosErrorResponse::internal(err, error_code, None)
    }

    fn internal_with_optional_vm_status_and_ledger_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: Option<aptos_types::vm_status::StatusCode>,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        if let Some(vs) = vm_status {
            AptosErrorResponse::internal_with_vm_status(err, error_code, vs, ledger_info)
        } else {
            AptosErrorResponse::internal(err, error_code, ledger_info)
        }
    }

    fn internal_with_vm_status<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: aptos_types::vm_status::StatusCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::internal_with_vm_status(err, error_code, vm_status, Some(ledger_info))
    }

    fn internal_from_aptos_error(aptos_error: AptosError, ledger_info: &LedgerInfo) -> Self {
        AptosErrorResponse::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            aptos_error,
            Some(ledger_info),
        )
    }
}

impl crate::response::ServiceUnavailableError for AptosErrorResponse {
    fn service_unavailable_with_code<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::service_unavailable(err, error_code, Some(ledger_info))
    }

    fn service_unavailable_with_code_no_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
    ) -> Self {
        AptosErrorResponse::service_unavailable(err, error_code, None)
    }

    fn service_unavailable_with_optional_vm_status_and_ledger_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: Option<aptos_types::vm_status::StatusCode>,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        if let Some(vs) = vm_status {
            let mut resp = AptosErrorResponse::service_unavailable(err, error_code, ledger_info);
            resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vs);
            resp
        } else {
            AptosErrorResponse::service_unavailable(err, error_code, ledger_info)
        }
    }

    fn service_unavailable_with_vm_status<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: aptos_types::vm_status::StatusCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        let mut resp = AptosErrorResponse::service_unavailable(err, error_code, Some(ledger_info));
        resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vm_status);
        resp
    }

    fn service_unavailable_from_aptos_error(
        aptos_error: AptosError,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::new(
            StatusCode::SERVICE_UNAVAILABLE,
            aptos_error,
            Some(ledger_info),
        )
    }
}

impl crate::response::PayloadTooLargeError for AptosErrorResponse {
    fn payload_too_large_with_code<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::payload_too_large(err, error_code, Some(ledger_info))
    }

    fn payload_too_large_with_code_no_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
    ) -> Self {
        AptosErrorResponse::payload_too_large(err, error_code, None)
    }

    fn payload_too_large_with_optional_vm_status_and_ledger_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: Option<aptos_types::vm_status::StatusCode>,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        if let Some(vs) = vm_status {
            let mut resp = AptosErrorResponse::payload_too_large(err, error_code, ledger_info);
            resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vs);
            resp
        } else {
            AptosErrorResponse::payload_too_large(err, error_code, ledger_info)
        }
    }

    fn payload_too_large_with_vm_status<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: aptos_types::vm_status::StatusCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        let mut resp = AptosErrorResponse::payload_too_large(err, error_code, Some(ledger_info));
        resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vm_status);
        resp
    }

    fn payload_too_large_from_aptos_error(
        aptos_error: AptosError,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::new(
            StatusCode::PAYLOAD_TOO_LARGE,
            aptos_error,
            Some(ledger_info),
        )
    }
}

impl crate::response::InsufficientStorageError for AptosErrorResponse {
    fn insufficient_storage_with_code<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::insufficient_storage(err, error_code, Some(ledger_info))
    }

    fn insufficient_storage_with_code_no_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
    ) -> Self {
        AptosErrorResponse::insufficient_storage(err, error_code, None)
    }

    fn insufficient_storage_with_optional_vm_status_and_ledger_info<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: Option<aptos_types::vm_status::StatusCode>,
        ledger_info: Option<&LedgerInfo>,
    ) -> Self {
        if let Some(vs) = vm_status {
            let mut resp = AptosErrorResponse::insufficient_storage(err, error_code, ledger_info);
            resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vs);
            resp
        } else {
            AptosErrorResponse::insufficient_storage(err, error_code, ledger_info)
        }
    }

    fn insufficient_storage_with_vm_status<Err: Display>(
        err: Err,
        error_code: AptosErrorCode,
        vm_status: aptos_types::vm_status::StatusCode,
        ledger_info: &LedgerInfo,
    ) -> Self {
        let mut resp = AptosErrorResponse::insufficient_storage(err, error_code, Some(ledger_info));
        resp.error = AptosError::new_with_vm_status(resp.error.message, error_code, vm_status);
        resp
    }

    fn insufficient_storage_from_aptos_error(
        aptos_error: AptosError,
        ledger_info: &LedgerInfo,
    ) -> Self {
        AptosErrorResponse::new(
            StatusCode::INSUFFICIENT_STORAGE,
            aptos_error,
            Some(ledger_info),
        )
    }
}
