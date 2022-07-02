// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{v2::types::LedgerResponse, USER_AGENT};
use anyhow::Result;
use aptos_api_types::LedgerInfo;
use reqwest::{Client as ReqwestClient, ClientBuilder as ReqwestClientBuilder};
use serde::de::DeserializeOwned;
use std::time::Duration;
use url::Url;

/// Builder for [`AptosClient`]
#[derive(Debug)]
pub struct AptosClientBuilder {
    base_url: Url,
    inner: ReqwestClientBuilder,
}

impl AptosClientBuilder {
    pub fn new(base_url: Url) -> Self {
        let inner = ReqwestClient::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(USER_AGENT)
            .cookie_store(true);
        Self { base_url, inner }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    pub fn build(self) -> Result<AptosClient> {
        Ok(AptosClient {
            base_url: self.base_url,
            inner: self.inner.build()?,
        })
    }
}

/// A client for the Aptos REST API
#[derive(Clone, Debug)]
pub struct AptosClient {
    base_url: Url,
    inner: ReqwestClient,
}

impl AptosClient {
    /// Make a GET request for a URL
    async fn get_url<T: DeserializeOwned>(&self, url: Url) -> Result<LedgerResponse<T>> {
        LedgerResponse::from_response(self.inner.get(url).send().await?).await
    }

    /// Make a POST request for a URL
    async fn post_url<T: DeserializeOwned, Body: Into<reqwest::Body>>(
        &self,
        url: Url,
        body: Body,
    ) -> Result<LedgerResponse<T>> {
        LedgerResponse::from_response(self.inner.post(url).body(body).send().await?).await
    }

    // -- General APIs --
    /// Get `LedgerInfo`
    async fn get_ledger_info(&self) -> Result<LedgerResponse<LedgerInfo>> {
        self.get_url(self.base_url.clone()).await
    }

    // -- Account APIs --
}
