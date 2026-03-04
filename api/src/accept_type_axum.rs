// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{accept_type::AcceptType, response_axum::AptosErrorResponse};
use aptos_api_types::mime_types::{BCS, JSON};
use axum::{
    extract::FromRequestParts,
    http::{header::ACCEPT, request::Parts},
};

#[axum::async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AcceptType {
    type Rejection = AptosErrorResponse;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let accept_header = parts
            .headers
            .get(ACCEPT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        for mime in accept_header.split(',').map(str::trim) {
            let mime_lower = mime.split(';').next().unwrap_or("").trim();
            if mime_lower == JSON {
                return Ok(AcceptType::Json);
            }
            if mime_lower == BCS {
                return Ok(AcceptType::Bcs);
            }
        }

        Ok(AcceptType::Json)
    }
}
