// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_api_types::mime_types::BCS;
use axum::{
    body::Bytes,
    extract::FromRequest,
    http::{header, Request, StatusCode},
    response::{IntoResponse, Response},
};

use crate::response_axum::AptosErrorResponse;

#[derive(Debug, Clone, Eq, PartialEq)]
#[allow(dead_code)]
pub struct BcsPayload(pub Vec<u8>);

impl std::ops::Deref for BcsPayload {
    type Target = Vec<u8>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[axum::async_trait]
impl<S: Send + Sync> FromRequest<S> for BcsPayload {
    type Rejection = AptosErrorResponse;

    async fn from_request(
        req: Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let bytes = Bytes::from_request(req, state).await.map_err(|e| {
            AptosErrorResponse::bad_request(
                format!("Failed to read request body: {}", e),
                aptos_api_types::AptosErrorCode::InvalidInput,
                None,
            )
        })?;
        Ok(Self(bytes.to_vec()))
    }
}

impl IntoResponse for BcsPayload {
    fn into_response(self) -> Response {
        (StatusCode::OK, [(header::CONTENT_TYPE, BCS)], self.0).into_response()
    }
}
