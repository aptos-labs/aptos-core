// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{error::FaucetClientError, Client, Result};
use velor_types::transaction::SignedTransaction;
use move_core_types::account_address::AccountAddress;
use reqwest::{Client as ReqwestClient, Response, Url};
use std::time::Duration;

pub struct FaucetClient {
    faucet_url: Url,
    inner: ReqwestClient,
    rest_client: Client,
    token: Option<String>,
}

impl FaucetClient {
    pub fn new(faucet_url: Url, rest_url: Url) -> Self {
        Self::new_from_rest_client(faucet_url, Client::new(rest_url))
    }

    pub fn new_for_testing(faucet_url: Url, rest_url: Url) -> Self {
        Self {
            faucet_url,
            inner: ReqwestClient::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            rest_client: Client::new(rest_url)
                // By default the path is prefixed with the version, e.g. `v1`.
                // The fake API used in the faucet tests doesn't have a
                // versioned API however, so we just set it to `/`.
                .version_path_base("/".to_string())
                .unwrap(),
            token: None,
        }
    }

    pub fn new_from_rest_client(faucet_url: Url, rest_client: Client) -> Self {
        Self {
            faucet_url,
            inner: ReqwestClient::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            rest_client,
            token: None,
        }
    }

    // Set auth token.
    pub fn with_auth_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    /// Create an account with zero balance.
    pub async fn create_account(&self, address: AccountAddress) -> Result<()> {
        let mut url = self.faucet_url.clone();
        url.set_path("mint");
        let query = format!("auth_key={}&amount=0&return_txns=true", address);
        url.set_query(Some(&query));

        let response = self.build_and_submit_request(url).await?;
        let status_code = response.status();
        let body = response.text().await.map_err(FaucetClientError::decode)?;
        if !status_code.is_success() {
            return Err(anyhow::anyhow!("body: {}", body));
        }

        let bytes = hex::decode(body).map_err(FaucetClientError::decode)?;
        let txns: Vec<SignedTransaction> =
            bcs::from_bytes(&bytes).map_err(FaucetClientError::decode)?;

        self.rest_client
            .wait_for_signed_transaction(&txns[0])
            .await
            .map_err(FaucetClientError::unknown)?;

        Ok(())
    }

    /// Fund an account with the given amount.
    pub async fn fund(&self, address: AccountAddress, amount: u64) -> Result<()> {
        let mut url = self.faucet_url.clone();
        url.set_path("mint");
        let query = format!("auth_key={}&amount={}&return_txns=true", address, amount);
        url.set_query(Some(&query));

        // Faucet returns the transaction that creates the account and needs to be waited on before
        // returning.
        let response = self.build_and_submit_request(url).await?;
        let status_code = response.status();
        let body = response.text().await.map_err(FaucetClientError::decode)?;
        if !status_code.is_success() {
            return Err(FaucetClientError::status(status_code.as_u16()).into());
        }

        let bytes = hex::decode(body).map_err(FaucetClientError::decode)?;
        let txns: Vec<SignedTransaction> =
            bcs::from_bytes(&bytes).map_err(FaucetClientError::decode)?;

        self.rest_client
            .wait_for_signed_transaction(&txns[0])
            .await
            .map_err(FaucetClientError::unknown)?;

        Ok(())
    }

    // Create and fund an account.
    pub async fn mint(&self, address: AccountAddress, amount: u64) -> Result<()> {
        self.create_account(address).await?;
        self.fund(address, amount).await?;

        Ok(())
    }

    // Helper to carry out requests.
    async fn build_and_submit_request(&self, url: Url) -> Result<Response> {
        // build request
        let mut request = self.inner.post(url).header("content-length", 0);
        if let Some(token) = &self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        // carry out and return response
        let response = request.send().await.map_err(FaucetClientError::request)?;
        Ok(response)
    }
}
