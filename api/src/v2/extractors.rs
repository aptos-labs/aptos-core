// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Custom Axum extractors for content-type negotiation in the v2 API.

use super::error::{ErrorCode, V2Error};
use axum::{
    body::Bytes,
    extract::FromRequest,
    http::{header::CONTENT_TYPE, Request},
};
use serde::Deserialize;

/// Versioned BCS request envelope.
/// The first byte(s) are a ULEB128-encoded enum discriminant (0 = V1, ...),
/// followed by the version-specific BCS-encoded payload.
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
pub enum Versioned<V1> {
    V1(V1),
    // Future: V2(V2Type), etc.
}

impl<V1: serde::de::DeserializeOwned> Versioned<V1> {
    #[allow(clippy::result_large_err)]
    pub fn from_bcs(bytes: &[u8]) -> Result<Self, V2Error> {
        bcs::from_bytes(bytes).map_err(|e| {
            V2Error::bad_request(
                ErrorCode::InvalidBcsVersion,
                format!("Failed to deserialize versioned BCS input: {}", e),
            )
        })
    }

    pub fn into_inner(self) -> V1 {
        match self {
            Versioned::V1(v) => v,
        }
    }
}

/// Extractor that reads the request body as either JSON or versioned BCS,
/// depending on Content-Type header.
pub enum JsonOrBcs<T: serde::de::DeserializeOwned> {
    Json(T),
    Bcs(Versioned<T>),
}

#[axum::async_trait]
impl<S, T> FromRequest<S> for JsonOrBcs<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned + Send + 'static,
{
    type Rejection = V2Error;

    async fn from_request(
        req: Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/json")
            .to_string();

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

        if content_type.contains("application/json") || content_type.contains("text/json") {
            let value = serde_json::from_slice(&bytes).map_err(|e| {
                V2Error::bad_request(ErrorCode::InvalidInput, format!("Invalid JSON: {}", e))
            })?;
            Ok(JsonOrBcs::Json(value))
        } else if content_type.contains("bcs") || content_type.contains("octet-stream") {
            let versioned = Versioned::from_bcs(&bytes)?;
            Ok(JsonOrBcs::Bcs(versioned))
        } else {
            Err(V2Error::bad_request(
                ErrorCode::InvalidInput,
                format!("Unsupported Content-Type: {}", content_type),
            ))
        }
    }
}

/// Extractor for BCS-only endpoints (like transaction submission).
/// Rejects JSON input.
pub struct BcsOnly<T: serde::de::DeserializeOwned>(pub Versioned<T>);

#[axum::async_trait]
impl<S, T> FromRequest<S> for BcsOnly<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned + Send + 'static,
{
    type Rejection = V2Error;

    async fn from_request(
        req: Request<axum::body::Body>,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let content_type = req
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.contains("json") {
            return Err(V2Error::bad_request(
                ErrorCode::InvalidInput,
                "This endpoint only accepts BCS input. JSON transaction submission is not supported in v2.",
            ));
        }

        let bytes = Bytes::from_request(req, state)
            .await
            .map_err(|e| V2Error::bad_request(ErrorCode::InvalidInput, e.to_string()))?;

        let versioned = Versioned::from_bcs(&bytes)?;
        Ok(BcsOnly(versioned))
    }
}
