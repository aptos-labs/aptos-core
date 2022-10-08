// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{error::FaucetClientError, Client, Result};
use aptos_crypto::HashValue;
use move_core_types::account_address::AccountAddress;
use reqwest::{Client as ReqwestClient, Url};
use std::time::{SystemTime, UNIX_EPOCH, Duration};


pub struct FaucetClient {
    faucet_url: Url,
    inner: ReqwestClient,
    rest_client: Client,
}

impl FaucetClient {
    pub fn new(faucet_url: Url, rest_url: Url) -> Self {
        Self {
            faucet_url,
            inner: ReqwestClient::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap(),
            rest_client: Client::new(rest_url),
        }
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
        }
    }

    /// Create an account with zero balance.
    pub async fn create_account(&self, address: AccountAddress) -> Result<()> {
        let mut url = self.faucet_url.clone();
        url.set_path("mint");
        let query = format!("address=0x{}&amount=0", address);
        url.set_query(Some(&query));

        // Faucet returns the transaction that creates the account and needs to be waited on before
        // returning.
        let response = self
            .inner
            .post(url)
            .header("content-type", "application/json; charset=utf-8")
            .header("content-length", 68)
            .send()
            .await
            .map_err(FaucetClientError::request)?;

        let status_code = response.status();
        let body = response.text().await.map_err(FaucetClientError::decode)?;
        if !status_code.is_success() {
            return Err(anyhow::anyhow!("body: {}", body));
        }

        let bytes = hex::decode(&body[2..body.len() - 2]).map_err(FaucetClientError::decode)?;

        let hash = HashValue::new(bytes.try_into().unwrap());
        let expiration_timestamp_secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().checked_add(20).unwrap();

        self.rest_client.wait_for_transaction_by_hash(
            hash, 
            expiration_timestamp_secs,
            None,
            None
        ).await?;

        Ok(())
    }

    /// Fund an account with the given amount.
    pub async fn fund(&self, address: AccountAddress, amount: u64) -> Result<()> {
        let mut url = self.faucet_url.clone();
        url.set_path("mint");
        let query = format!("address=0x{}&amount={}", address, amount);
        url.set_query(Some(&query));

        // Faucet returns the transaction that creates the account and needs to be waited on before
        // returning.
        let response = self
            .inner
            .post(url)
            .header("content-type", "application/json; charset=utf-8")
            .header("content-length", 68)
            .send()
            .await
            .map_err(FaucetClientError::request)?;

        let status_code = response.status();
        let body = response.text().await.map_err(FaucetClientError::decode)?;
        if !status_code.is_success() {
            return Err(anyhow::anyhow!("body: {}", body));
        }

        let bytes = hex::decode(&body[2..body.len() - 2]).map_err(FaucetClientError::decode)?;

        let hash = HashValue::new(bytes.try_into().unwrap());
        let expiration_timestamp_secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs().checked_add(20).unwrap();

        self.rest_client.wait_for_transaction_by_hash(
            hash, 
            expiration_timestamp_secs,
            None,
            None
        ).await?;

        Ok(())
    }

    // Create and fund an account.
    pub async fn mint(&self, address: AccountAddress, amount: u64) -> Result<()> {
        self.create_account(address).await?;
        self.fund(address, amount).await?;

        Ok(())
    }
}
