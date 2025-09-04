// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::State;
use velor_api_types::VelorError;
use reqwest::StatusCode;
use thiserror::Error;

#[derive(Debug)]
pub struct FaucetClientError {
    inner: Box<Inner>,
}

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(Debug)]
struct Inner {
    kind: Kind,
    source: Option<BoxError>,
}

#[derive(Debug)]
enum Kind {
    HttpStatus(u16),
    Timeout,
    Request,
    RpcResponse,
    ChainId,
    StaleResponse,
    Batch,
    Decode,
    InvalidProof,
    NeedSync,
    StateStore,
    Unknown,
}

impl FaucetClientError {
    pub fn is_retriable(&self) -> bool {
        match self.inner.kind {
            // internal server errors are retriable
            Kind::HttpStatus(status) => (500..=599).contains(&status),
            Kind::Timeout | Kind::StaleResponse | Kind::NeedSync => true,
            Kind::RpcResponse
            | Kind::Request
            | Kind::ChainId
            | Kind::Batch
            | Kind::Decode
            | Kind::InvalidProof
            | Kind::StateStore
            | Kind::Unknown => false,
        }
    }

    pub fn is_need_sync(&self) -> bool {
        matches!(self.inner.kind, Kind::NeedSync)
    }

    //
    // Private Constructors
    //

    fn new<E: Into<BoxError>>(kind: Kind, source: Option<E>) -> Self {
        Self {
            inner: Box::new(Inner {
                kind,
                source: source.map(Into::into),
            }),
        }
    }

    pub fn status(status: u16) -> Self {
        Self::new(Kind::HttpStatus(status), None::<FaucetClientError>)
    }

    pub fn timeout<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Timeout, Some(e))
    }

    pub fn rpc_response<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::RpcResponse, Some(e))
    }

    pub fn batch<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Batch, Some(e))
    }

    pub fn decode<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Decode, Some(e))
    }

    pub fn encode<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Decode, Some(e))
    }

    pub fn invalid_proof<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::InvalidProof, Some(e))
    }

    pub fn state_store<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::StateStore, Some(e))
    }

    pub fn need_sync<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::NeedSync, Some(e))
    }

    pub fn unknown<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Unknown, Some(e))
    }

    pub fn request<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::Request, Some(e))
    }

    pub fn chain_id(expected: u8, received: u8) -> Self {
        Self::new(
            Kind::ChainId,
            Some(format!("expected: {} received: {}", expected, received)),
        )
    }

    pub fn stale<E: Into<BoxError>>(e: E) -> Self {
        Self::new(Kind::StaleResponse, Some(e))
    }
}

impl std::fmt::Display for FaucetClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for FaucetClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.inner.source.as_ref().map(|e| &**e as _)
    }
}

impl From<serde_json::Error> for FaucetClientError {
    fn from(e: serde_json::Error) -> Self {
        Self::decode(e)
    }
}

#[derive(Debug, Error)]
pub enum RestError {
    #[error("API error {0}")]
    Api(VelorErrorResponse),
    #[error("BCS ser/de error {0}")]
    Bcs(bcs::Error),
    #[error("JSON er/de error {0}")]
    Json(serde_json::Error),
    #[error("URL Parse error {0}")]
    UrlParse(url::ParseError),
    #[error("Timeout waiting for transaction {0}")]
    Timeout(&'static str),
    #[error("Unknown error {0}")]
    Unknown(anyhow::Error),
    #[error("HTTP error {0}: {1}")]
    Http(StatusCode, reqwest::Error),
}

impl From<(VelorError, Option<State>, StatusCode)> for RestError {
    fn from((error, state, status_code): (VelorError, Option<State>, StatusCode)) -> Self {
        Self::Api(VelorErrorResponse {
            error,
            state,
            status_code,
        })
    }
}

impl From<bcs::Error> for RestError {
    fn from(err: bcs::Error) -> Self {
        Self::Bcs(err)
    }
}

impl From<url::ParseError> for RestError {
    fn from(err: url::ParseError) -> Self {
        Self::UrlParse(err)
    }
}

impl From<serde_json::Error> for RestError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<anyhow::Error> for RestError {
    fn from(err: anyhow::Error) -> Self {
        Self::Unknown(err)
    }
}

impl From<reqwest::Error> for RestError {
    fn from(err: reqwest::Error) -> Self {
        if let Some(status) = err.status() {
            RestError::Http(status, err)
        } else {
            RestError::Unknown(err.into())
        }
    }
}

#[derive(Debug)]
pub struct VelorErrorResponse {
    pub error: VelorError,
    pub state: Option<State>,
    pub status_code: StatusCode,
}

impl std::fmt::Display for VelorErrorResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}
