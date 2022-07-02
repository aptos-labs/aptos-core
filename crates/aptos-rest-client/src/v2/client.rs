// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    v2::types::{LedgerResponse, Page},
    Account, AccountAddress, AptosVersion, Balance, Resource, BCS_CONTENT_TYPE, USER_AGENT,
};
use anyhow::{anyhow, Result};
use aptos_api_types::{LedgerInfo, MoveModuleBytecode, Transaction};
use aptos_crypto::HashValue;
use aptos_types::account_config::aptos_root_address;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use reqwest::{
    header::CONTENT_TYPE, Client as ReqwestClient, ClientBuilder as ReqwestClientBuilder,
};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;

const DEFAULT_TXN_TIMEOUT: Duration = Duration::from_secs(60);
const DEFAULT_TXN_RETRY_DELAY: Duration = Duration::from_millis(500);

/// Builder for [`AptosClient`]
#[derive(Debug)]
pub struct AptosClientBuilder {
    base_url: Url,
    inner: ReqwestClientBuilder,
    txn_timeout: Duration,
    txn_retry_delay: Duration,
}

impl AptosClientBuilder {
    pub fn new(base_url: Url) -> Self {
        let inner = ReqwestClient::builder()
            .timeout(Duration::from_secs(10))
            .user_agent(USER_AGENT)
            .cookie_store(true);
        Self {
            base_url,
            inner,
            txn_timeout: DEFAULT_TXN_TIMEOUT,
            txn_retry_delay: DEFAULT_TXN_RETRY_DELAY,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.inner = self.inner.timeout(timeout);
        self
    }

    pub fn build(self) -> Result<AptosClient> {
        Ok(AptosClient {
            base_url: self.base_url,
            inner: self.inner.build()?,
            txn_timeout: self.txn_timeout,
            txn_retry_delay: self.txn_retry_delay,
        })
    }
}

/// A client for the Aptos REST API
#[derive(Clone, Debug)]
pub struct AptosClient {
    base_url: Url,
    inner: ReqwestClient,
    txn_timeout: Duration,
    txn_retry_delay: Duration,
}

impl AptosClient {
    /// Make a GET request for a URL
    async fn get_url<T: DeserializeOwned>(&self, url: Url) -> Result<LedgerResponse<T>> {
        LedgerResponse::from_response(self.inner.get(url).send().await?).await
    }

    /// Make a POST request for a URL in JSON
    async fn post_url_json<T: DeserializeOwned, Body: Serialize>(
        &self,
        url: Url,
        body: &Body,
    ) -> Result<LedgerResponse<T>> {
        LedgerResponse::from_response(self.inner.post(url).json(body).send().await?).await
    }

    /// Make a POST request for a URL in BCS
    async fn post_url_bcs<T: DeserializeOwned, Body: Serialize>(
        &self,
        url: Url,
        body: &Body,
    ) -> Result<LedgerResponse<T>> {
        LedgerResponse::from_response(
            self.inner
                .post(url)
                .header(CONTENT_TYPE, BCS_CONTENT_TYPE)
                .body(bcs::to_bytes(body)?)
                .send()
                .await?,
        )
        .await
    }

    // -- General APIs --
    /// Get the `LedgerInfo` current state of the network
    pub async fn get_ledger_info(&self) -> Result<LedgerResponse<LedgerInfo>> {
        self.get_url(self.base_url.clone()).await
    }

    pub async fn get_aptos_version(&self, version: Option<u64>) -> Result<AptosVersion> {
        self.get_typed_resource::<AptosVersion>(
            aptos_root_address(),
            "0x1::Version::Version",
            version,
        )
        .await
    }

    /// Health check the endpoint
    pub async fn health_check(&self, seconds: u64) -> Result<()> {
        let response = self
            .inner
            .get(self.base_url.join("-/healthy")?)
            .query(&[("duration_secs", seconds)])
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!("health check failed"));
        }

        Ok(())
    }

    // -- Account APIs --
    /// Get an [`Account`]'s attributes
    pub async fn get_account(
        &self,
        account_address: AccountAddress,
    ) -> Result<LedgerResponse<Account>> {
        self.get_url(
            self.base_url
                .join(&format!("/accounts/{}", account_address.to_hex_literal()))?,
        )
        .await
    }

    /// Get all [`Resource`]s associated with an account at a version
    ///
    /// If no version is provided, it will grab the latest version
    pub async fn get_account_resources(
        &self,
        account_address: AccountAddress,
        version: Option<u64>,
    ) -> Result<LedgerResponse<Vec<Resource>>> {
        let path = with_version(
            format!("/accounts/{}/resources", account_address.to_hex_literal()),
            version,
        );
        self.get_url(self.base_url.join(&path)?).await
    }

    /// Get a specific [`Resource`] associated with an account at a version
    ///
    /// If no version is provided, it will grab the latest version
    pub async fn get_account_resource(
        &self,
        account_address: AccountAddress,
        resource_type: &str,
        version: Option<u64>,
    ) -> Result<LedgerResponse<Option<Resource>>> {
        const ENCODING_CHARS: &AsciiSet = &CONTROLS.add(b'<').add(b'>');
        let resource_type = utf8_percent_encode(resource_type, ENCODING_CHARS).to_string();

        let path = with_version(
            format!(
                "/accounts/{}/resource/{}",
                account_address.to_hex_literal(),
                resource_type
            ),
            version,
        );
        self.get_url(self.base_url.join(&path)?).await
    }

    /// Get an account's [`Resource`] and convert it to the typed `T`
    ///
    /// TODO: This should probably be BCS encoded as well
    pub async fn get_typed_resource<T: DeserializeOwned>(
        &self,
        account_address: AccountAddress,
        resource_type: &str,
        version: Option<u64>,
    ) -> Result<T> {
        let response = self
            .get_account_resource(account_address, resource_type, version)
            .await?;
        if let Some(resource) = response.into_inner() {
            serde_json::from_value(resource.data)
                .map_err(|e| anyhow!("deserializing {} failed: {}", resource_type, e))
        } else {
            Err(anyhow!(
                "Failed to find resource {} in account {}",
                resource_type,
                account_address
            ))
        }
    }

    /// Get all [`MoveModuleBytecode`]s associated with an account at a version
    ///
    /// If no version is provided, it will grab the latest version
    pub async fn get_account_modules(
        &self,
        account_address: AccountAddress,
        version: Option<u64>,
    ) -> Result<LedgerResponse<Vec<MoveModuleBytecode>>> {
        let path = with_version(
            format!("/accounts/{}/modules", account_address.to_hex_literal()),
            version,
        );
        self.get_url(self.base_url.join(&path)?).await
    }

    /// Retrieves transactions associated with an account
    ///
    /// Takes optional paging information, that can be used to page through transactions
    pub async fn get_account_transactions(
        &self,
        account_address: AccountAddress,
        page: Option<Page>,
    ) -> Result<LedgerResponse<Vec<Transaction>>> {
        let path = with_page(
            format!(
                "/accounts/{}/transactions",
                account_address.to_hex_literal()
            ),
            page,
        );
        self.get_url(self.base_url.join(&path)?).await
    }

    pub async fn get_account_balance(
        &self,
        account_address: AccountAddress,
        version: Option<u64>,
    ) -> Result<Balance> {
        self.get_typed_resource(
            account_address,
            "0x1::Coin::CoinStore<0x1::TestCoin::TestCoin>",
            version,
        )
        .await
    }

    // -- Transaction APIs --
    /// Get a transaction by it's hash value
    pub async fn get_transaction_by_hash(
        &self,
        hash: HashValue,
    ) -> Result<LedgerResponse<Transaction>> {
        self.get_transaction(hash.to_hex_literal()).await
    }

    /// Get a transaction by it's ledger version
    pub async fn get_transaction_by_version(
        &self,
        version: u64,
    ) -> Result<LedgerResponse<Transaction>> {
        self.get_transaction(version.to_string()).await
    }

    async fn get_transaction(
        &self,
        version_or_hash: String,
    ) -> Result<LedgerResponse<Transaction>> {
        self.get_url(
            self.base_url
                .join(&format!("transactions/{}", version_or_hash))?,
        )
        .await
    }

    // -- Table APIs --
    pub async fn get_table_item<K: Serialize>(
        &self,
        table_handle: u128,
        key_type: &str,
        value_type: &str,
        key: K,
    ) -> Result<LedgerResponse<Value>> {
        let data = json!({
            "key_type": key_type,
            "value_type": value_type,
            "key": json!(key),
        });

        self.post_url_json(
            self.base_url
                .join(&format!("tables/{}/item", table_handle))?,
            &data,
        )
        .await
    }

    // -- Submit APIs --
}

/// Appends version query arg if applicable
fn with_version(path: String, version: Option<u64>) -> String {
    if let Some(version) = version {
        format!("{}?version={}", path, version)
    } else {
        path
    }
}

/// Appends page query args if applicable
fn with_page(path: String, page: Option<Page>) -> String {
    let query_params = match page {
        Some(Page {
            start: Some(start),
            limit: Some(limit),
        }) => Some(format!("start={}&limit={}", start, limit)),
        Some(Page {
            start: None,
            limit: Some(limit),
        }) => Some(format!("limit={}", limit)),
        Some(Page {
            start: Some(start),
            limit: None,
        }) => Some(format!("start={}", start)),
        _ => None,
    };
    if let Some(query_params) = query_params {
        format!("{}?{}", path, query_params)
    } else {
        path
    }
}
