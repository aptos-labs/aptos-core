// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A command to create a new account on-chain
//!
//! TODO: Examples
//!

use crate::{
    common::types::{
        account_address_from_public_key, EncodingOptions, ExtractPublicKey, NodeOptions,
        PrivateKeyInputOptions, PublicKeyInputOptions,
    },
    CliResult, Error as CommonError,
};
use anyhow::Error;
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_rest_client::{Client as RestClient, Response, Transaction};
use aptos_sdk::{
    transaction_builder::TransactionFactory,
    types::{chain_id::ChainId, LocalAccount},
};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use clap::Parser;
use reqwest;

/// Command to create a new account on-chain
///
#[derive(Debug, Parser)]
pub struct CreateAccount {
    #[clap(flatten)]
    private_key_input_options: PrivateKeyInputOptions,

    #[clap(flatten)]
    encoding_options: EncodingOptions,

    #[clap(flatten)]
    node: NodeOptions,

    #[clap(flatten)]
    public_key_input_options: PublicKeyInputOptions,

    /// Chain ID
    #[clap(long)]
    chain_id: u8,

    /// Flag for using faucet
    #[clap(long)]
    use_faucet: bool,

    /// Initial coins to fund when using the faucet
    #[clap(long, default_value = "10000")]
    initial_coins: u64,
}

impl CreateAccount {
    async fn get_account(
        &self,
        account: AccountAddress,
    ) -> Result<serde_json::Value, reqwest::Error> {
        reqwest::get(format!("{}accounts/{}", self.node.url, account))
            .await?
            .json()
            .await
    }

    async fn get_sequence_number(&self, account: AccountAddress) -> Result<u64, CommonError> {
        let account_response = self
            .get_account(account)
            .await
            .map_err(|err| CommonError::UnexpectedError(err.to_string()))?;
        let sequence_number = &account_response["sequence_number"];
        match sequence_number.as_str() {
            Some(number) => Ok(number.parse::<u64>().unwrap()),
            None => Err(CommonError::UnexpectedError(
                "Sequence number not found".to_string(),
            )),
        }
    }

    async fn post_account(
        &self,
        address: AccountAddress,
        sender_key: Ed25519PrivateKey,
        sender_address: AccountAddress,
        sequence_number: u64,
    ) -> Result<Response<Transaction>, Error> {
        let client = RestClient::new(reqwest::Url::clone(&self.node.url));
        let chain_id = ChainId::new(self.chain_id);
        let transaction_factory = TransactionFactory::new(chain_id)
            .with_gas_unit_price(1)
            .with_max_gas_amount(1000);
        let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
        let transaction = sender_account.sign_with_transaction_builder(
            transaction_factory
                .payload(aptos_stdlib::encode_create_account_script_function(address)),
        );
        client.submit_and_wait(&transaction).await
    }

    async fn create_account_with_faucet(self, address: AccountAddress) -> Result<String, Error> {
        let response = reqwest::Client::new()
            // TODO: Currently, we are just using mint 0 to create an account using the faucet
            // We should make a faucet endpoint for creating an account
            .post(format!(
                "{}/mint?amount={}&auth_key={}",
                "https://faucet.devnet.aptoslabs.com", self.initial_coins, address
            ))
            .send()
            .await?;
        if response.status() == 200 {
            Ok(response.status().to_string())
        } else {
            Err(Error::new(CommonError::UnexpectedError(format!(
                "Faucet issue: {}",
                response.status()
            ))))
        }
    }

    async fn create_account_with_key(self, address: AccountAddress) -> Result<String, Error> {
        let sender_private_key = self
            .private_key_input_options
            .extract_private_key(self.encoding_options.encoding)?;
        let sender_public_key = sender_private_key.public_key();
        let sender_address = account_address_from_public_key(&sender_public_key);
        let sequence_number = self.get_sequence_number(sender_address).await;
        match sequence_number {
            Ok(sequence_number) => self
                .post_account(address, sender_private_key, sender_address, sequence_number)
                .await
                .map(|_| "Success".to_string()),
            Err(err) => Err(Error::new(err)),
        }
    }

    async fn execute_inner(self, address: AccountAddress) -> Result<String, Error> {
        if self.use_faucet {
            self.create_account_with_faucet(address).await
        } else {
            self.create_account_with_key(address).await
        }
    }

    pub async fn execute(self) -> CliResult {
        let public_key_to_create = self
            .public_key_input_options
            .extract_public_key(self.encoding_options.encoding)
            .map_err(|err| err.to_string())?;
        let new_address = account_address_from_public_key(&public_key_to_create);
        self.execute_inner(new_address)
            .await
            .map(|_| format!("Account Created at {}", new_address))
            .map_err(|err| err.to_string())
    }
}
