// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{error::Error, Client, Result};
use diem_types::{
    account_address::AccountAddress,
    transaction::{authenticator::AuthenticationKey, SignedTransaction},
};
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

    pub fn create_account(
        &self,
        authentication_key: AuthenticationKey,
        currency_code: &str,
    ) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let mut url = Url::parse(&self.faucet_url).map_err(Error::request)?;
        url.set_path("accounts");
        let query = format!(
            "authentication-key={}&currency={}",
            authentication_key, currency_code
        );
        url.set_query(Some(&query));

        let response = client.post(url).send().map_err(Error::request)?;
        let status_code = response.status();
        let body = response.text().map_err(Error::decode)?;
        if !status_code.is_success() {
            return Err(Error::status(status_code.as_u16()).into());
        }

        let bytes = hex::decode(body).map_err(Error::decode)?;
        let txn: SignedTransaction = bcs::from_bytes(&bytes).map_err(Error::decode)?;

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.rest_client.wait_for_signed_transaction(&txn))
            .map_err(Error::unknown)?;

        Ok(())
    }

    pub fn fund(&self, address: AccountAddress, currency_code: &str, amount: u64) -> Result<()> {
        let client = reqwest::blocking::Client::new();
        let mut url = Url::parse(&self.faucet_url).map_err(Error::request)?;
        url.set_path(&format!("accounts/{}/fund", address));
        let query = format!("currency={}&amount={}", currency_code, amount);
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
        let txn: SignedTransaction = bcs::from_bytes(&bytes).map_err(Error::decode)?;

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.rest_client.wait_for_signed_transaction(&txn))
            .map_err(Error::unknown)?;

        Ok(())
    }

    pub fn mint(
        &self,
        currency_code: &str,
        auth_key: AuthenticationKey,
        amount: u64,
    ) -> Result<()> {
        self.create_account(auth_key, currency_code)?;
        self.fund(auth_key.derived_address(), currency_code, amount)?;

        Ok(())
    }
}
