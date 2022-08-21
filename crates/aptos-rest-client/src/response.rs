// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::state::State;
use aptos_api_types::AptosError;
use reqwest::StatusCode;
use std::fmt::Formatter;
use thiserror::Error;

#[derive(Debug)]
pub struct Response<T> {
    inner: T,
    state: State,
}

impl<T> Response<T> {
    pub fn new(inner: T, state: State) -> Self {
        Self { inner, state }
    }

    pub fn inner(&self) -> &T {
        &self.inner
    }

    pub fn into_inner(self) -> T {
        self.inner
    }

    pub fn state(&self) -> &State {
        &self.state
    }

    pub fn into_parts(self) -> (T, State) {
        (self.inner, self.state)
    }

    pub fn and_then<U, E, F>(self, f: F) -> Result<Response<U>, E>
    where
        F: FnOnce(T) -> Result<U, E>,
    {
        let (inner, state) = self.into_parts();
        match f(inner) {
            Ok(new_inner) => Ok(Response::new(new_inner, state)),
            Err(err) => Err(err),
        }
    }

    pub fn map<U, F>(self, f: F) -> Response<U>
    where
        F: FnOnce(T) -> U,
    {
        let (inner, state) = self.into_parts();
        Response::new(f(inner), state)
    }
}

#[derive(Debug, Error)]
pub enum RestError {
    #[error("API error {0}")]
    Api(AptosErrorResponse),
    #[error("BCS ser/de error {0}")]
    Bcs(bcs::Error),
    #[error("JSON er/de error {0}")]
    Json(serde_json::Error),
    #[error("Web client error {0}")]
    WebClient(reqwest::Error),
    #[error("URL Parse error {0}")]
    UrlParse(url::ParseError),
    #[error("Timeout waiting for transaction {0}")]
    Timeout(&'static str),
    #[error("Unknown error {0}")]
    Unknown(anyhow::Error),
}

impl From<(AptosError, Option<State>, StatusCode)> for RestError {
    fn from((error, state, status_code): (AptosError, Option<State>, StatusCode)) -> Self {
        Self::Api(AptosErrorResponse {
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

impl From<reqwest::Error> for RestError {
    fn from(err: reqwest::Error) -> Self {
        Self::WebClient(err)
    }
}

impl From<anyhow::Error> for RestError {
    fn from(err: anyhow::Error) -> Self {
        Self::Unknown(err)
    }
}

#[derive(Debug)]
pub struct AptosErrorResponse {
    pub error: AptosError,
    pub state: Option<State>,
    pub status_code: StatusCode,
}

impl std::fmt::Display for AptosErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}
