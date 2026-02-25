// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_api_types::{AptosError, AptosErrorCode};
use axum::{
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
};

pub fn handle_panic(err: Box<dyn std::any::Any + Send + 'static>) -> Response {
    aptos_logger::error!("Panic captured: {:?}", err);
    let error = AptosError::new_with_error_code("internal error", AptosErrorCode::InternalError);
    let json = serde_json::to_vec(&error).unwrap_or_default();
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        [(CONTENT_TYPE, "application/json")],
        json,
    )
        .into_response()
}

pub async fn handle_404() -> Response {
    let error =
        AptosError::new_with_error_code("not found", AptosErrorCode::WebFrameworkError);
    let json = serde_json::to_vec(&error).unwrap_or_default();
    (
        StatusCode::NOT_FOUND,
        [(CONTENT_TYPE, "application/json")],
        json,
    )
        .into_response()
}
