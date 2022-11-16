// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{self, Debug};

use aptos_crypto::noise::NoiseError;
use aptos_rest_client::error::RestError;
use aptos_types::{chain_id::ChainId, PeerId};
use debug_ignore::DebugIgnore;
use gcp_bigquery_client::{
    error::BQError,
    model::table_data_insert_all_response_insert_errors::TableDataInsertAllResponseInsertErrors,
};
use thiserror::Error as ThisError;
use warp::{http::StatusCode, reject::Reject};

#[derive(Debug, ThisError)]
pub(crate) enum AuthError {
    #[error("invalid public key")]
    InvalidServerPublicKey,
    #[error("error performing noise handshake")]
    NoiseHandshakeError(NoiseError),
    #[error("public key not found in peer keys")]
    PeerPublicKeyNotFound,
    #[error("public key does not match identity")]
    PublicKeyMismatch,
    #[error("validator set unavailable for chain")]
    ValidatorSetUnavailable,
    #[error("unable to authenticate")]
    CreateJwtError(DebugIgnore<jsonwebtoken::errors::Error>),
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(val: jsonwebtoken::errors::Error) -> Self {
        Self::CreateJwtError(DebugIgnore(val))
    }
}

#[derive(Debug, ThisError)]
pub(crate) enum JwtAuthError {
    #[error("invalid authorization token")]
    InvalidAuthToken,
    #[error("expired authorization token")]
    ExpiredAuthToken,
    #[error("access denied to this resource")]
    AccessDenied,
    #[error("invalid authorization header: {0}")]
    InvalidAuthHeader(DebugIgnore<String>),
}

impl From<String> for JwtAuthError {
    fn from(val: String) -> Self {
        Self::InvalidAuthHeader(DebugIgnore(val))
    }
}

#[derive(Debug, ThisError)]
pub(crate) enum CustomEventIngestError {
    #[error("user_id {0} in event does not match peer_id {1}")]
    InvalidEvent(String, PeerId),
    #[error("no events in payload")]
    EmptyPayload,
    #[error("invalid payload timestamp: {0}")]
    InvalidTimestamp(String),
    #[error("unable to insert row into big query")]
    BigQueryClientError(DebugIgnore<BQError>),
    #[error("invalid payload schema: {0}")]
    BigQueryInsertError(DebugIgnore<TableDataInsertAllResponseInsertErrors>),
    #[error("{0}")]
    Other(DebugIgnore<anyhow::Error>),
}

impl From<BQError> for CustomEventIngestError {
    fn from(err: BQError) -> Self {
        Self::BigQueryClientError(DebugIgnore(err))
    }
}

impl From<TableDataInsertAllResponseInsertErrors> for CustomEventIngestError {
    fn from(err: TableDataInsertAllResponseInsertErrors) -> Self {
        Self::BigQueryInsertError(DebugIgnore(err))
    }
}

impl From<anyhow::Error> for CustomEventIngestError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(DebugIgnore(err))
    }
}

#[derive(Debug, ThisError)]
pub(crate) enum LogIngestError {
    #[error(
        "unexpected payload body. Payload should be an array of strings possibly in gzip format"
    )]
    UnexpectedPayloadBody,
    #[error("unexpected content encoding. Supported encodings are: gzip")]
    UnexpectedContentEncoding,
    #[error("unable to ingest logs")]
    IngestionError,
}

#[derive(Debug, ThisError)]
pub(crate) enum MetricsIngestError {
    #[error("unable to ingest metrics")]
    IngestionError,
}

#[derive(Debug, ThisError)]
pub(crate) enum ValidatorCacheUpdateError {
    #[error("invalid url")]
    InvalidUrl,
    #[error("request error")]
    RestError(#[source] RestError),
    #[error("chain id mismatch")]
    ChainIdMismatch,
    #[error("both peer set empty")]
    BothPeerSetEmpty,
    #[error("validator set empty")]
    ValidatorSetEmpty,
    #[error("vfn set empty")]
    VfnSetEmpty,
}

#[derive(Debug, ThisError)]
pub(crate) enum ServiceErrorCode {
    #[error("authentication error: {0}")]
    AuthError(AuthError, ChainId),
    #[error("custom event ingest error: {0}")]
    CustomEventIngestError(#[from] CustomEventIngestError),
    #[error("authorization error: {0}")]
    JwtAuthError(#[from] JwtAuthError),
    #[error("log ingest error: {0}")]
    LogIngestError(#[from] LogIngestError),
    #[error("metrics ingest error: {0}")]
    MetricsIngestError(#[from] MetricsIngestError),
}

#[derive(Debug)]
pub(crate) struct ServiceError {
    http_code: StatusCode,
    error_code: ServiceErrorCode,
}

impl ServiceError {
    pub(crate) fn new(http_code: StatusCode, error_code: ServiceErrorCode) -> Self {
        Self {
            http_code,
            error_code,
        }
    }

    pub(crate) fn bad_request(error_code: ServiceErrorCode) -> Self {
        Self::new(StatusCode::BAD_REQUEST, error_code)
    }

    pub(crate) fn unauthorized(error_code: ServiceErrorCode) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, error_code)
    }

    pub(crate) fn forbidden(error_code: ServiceErrorCode) -> Self {
        Self::new(StatusCode::FORBIDDEN, error_code)
    }

    pub(crate) fn internal(error_code: ServiceErrorCode) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, error_code)
    }

    pub(crate) fn http_status_code(&self) -> StatusCode {
        self.http_code
    }

    #[cfg(test)]
    pub(crate) fn error_as_string(&self) -> String {
        self.error_code.to_string()
    }

    pub(crate) fn error_code(&self) -> &ServiceErrorCode {
        &self.error_code
    }
}

impl Reject for ServiceError {}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.http_code, self.error_code)?;
        Ok(())
    }
}
