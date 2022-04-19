// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A command to create a new account on-chain
//!
//! TODO: Examples
//!

use crate::common::types::{
    account_address_from_public_key, ExtractPublicKey, PublicKeyInputOptions,
};
use crate::{
    common::types::{EncodingOptions, NodeOptions, PrivateKeyInputOptions},
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
    chain_id: u8,
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

    async fn execute_inner(self) -> Result<String, Error> {
        let sender_private_key = self
            .private_key_input_options
            .extract_private_key(self.encoding_options.encoding)?;
        let sender_public_key = sender_private_key.public_key();
        let sender_address = account_address_from_public_key(&sender_public_key);
        let sequence_number = self.get_sequence_number(sender_address).await;

        let public_key_to_create = self
            .public_key_input_options
            .extract_public_key(self.encoding_options.encoding)?;
        let new_address = account_address_from_public_key(&public_key_to_create);
        match sequence_number {
            Ok(sequence_number) => self
                .post_account(
                    new_address,
                    sender_private_key,
                    sender_address,
                    sequence_number,
                )
                .await
                .map(|_| format!("Account Created at {}", new_address)),
            Err(err) => Err(Error::new(err)),
        }
    }

    pub async fn execute(self) -> CliResult {
        self.execute_inner().await.map_err(|err| err.to_string())
    }
}
