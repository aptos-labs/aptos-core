// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{error::Error, Client, Result};
use aptos_types::transaction::SignedTransaction;
use move_deps::move_core_types::account_address::AccountAddress;
use reqwest::Url;

pub struct FaucetClient {
    faucet_url: String,
    rest_client: Client,
}

impl FaucetClient {
    pub fn new(faucet_url: String, rest_url: String) -> Self {
        Self {
            faucet_url,
            rest_client: Client::new(Url::parse(&rest_url).expect("Unable to parse rest url")),
        }
    }

    pub fn create_account(&self, address: AccountAddress) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let mut url = Url::parse(&self.faucet_url).map_err(Error::request)?;
        url.set_path("mint");
        let query = format!("auth_key={}&amount=0&return_txns=true", address);
        url.set_query(Some(&query));

        let response = client.post(url).send().map_err(Error::request)?;
        let status_code = response.status();
        let body = response.text().map_err(Error::decode)?;
        if !status_code.is_success() {
            return Err(anyhow::anyhow!("body: {}", body));
        }

        let bytes = hex::decode(body).map_err(Error::decode)?;
        let txns: Vec<SignedTransaction> = bcs::from_bytes(&bytes).map_err(Error::decode)?;

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.rest_client.wait_for_signed_transaction(&txns[0]))
            .map_err(Error::unknown)?;

        Ok(())
    }

    pub fn fund(&self, address: AccountAddress, amount: u64) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let mut url = Url::parse(&self.faucet_url).map_err(Error::request)?;
        url.set_path("mint");
        let query = format!("auth_key={}&amount={}&return_txns=true", address, amount);
        url.set_query(Some(&query));

        // Faucet returns the transaction that creates the account and needs to be waited on before
        // returning.
        let response = client.post(url).send().map_err(Error::request)?;
        let status_code = response.status();
        let body = response.text().map_err(Error::decode)?;
        if !status_code.is_success() {
            return Err(Error::status(status_code.as_u16()).into());
        }

        let bytes = hex::decode(body).map_err(Error::decode)?;
        let txns: Vec<SignedTransaction> = bcs::from_bytes(&bytes).map_err(Error::decode)?;

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.rest_client.wait_for_signed_transaction(&txns[0]))
            .map_err(Error::unknown)?;

        Ok(())
    }

    pub fn mint(&self, address: AccountAddress, amount: u64) -> Result<()> {
        self.create_account(address)?;
        self.fund(address, amount)?;

        Ok(())
    }
}
