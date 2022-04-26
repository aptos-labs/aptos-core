// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! A command to create a new account on-chain
//!
//! TODO: Examples
//!

use crate::common::{
    init::DEFAULT_FAUCET_URL,
    types::{
        account_address_from_public_key, CliConfig, CliError, CliTypedResult, EncodingOptions,
        ExtractPublicKey, PublicKeyInputOptions, WriteTransactionOptions,
    },
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey};
use aptos_rest_client::{Client as RestClient, Response, Transaction};
use aptos_sdk::{transaction_builder::TransactionFactory, types::LocalAccount};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use clap::Parser;
use reqwest::{self, Url};

/// Command to create a new account on-chain
///
#[derive(Debug, Parser)]
pub struct CreateAccount {
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    write_options: WriteTransactionOptions,
    #[clap(flatten)]
    public_key_options: PublicKeyInputOptions,
    /// Flag for using faucet to create the account
    #[clap(long)]
    use_faucet: bool,
    /// URL for the faucet
    #[clap(long)]
    faucet_url: Option<Url>,
    /// Initial coins to fund when using the faucet
    #[clap(long, default_value = "10000")]
    initial_coins: u64,
}

impl CreateAccount {
    pub async fn execute(self) -> CliTypedResult<String> {
        let public_key_to_create = self
            .public_key_options
            .extract_public_key(self.encoding_options.encoding)?;
        let address = account_address_from_public_key(&public_key_to_create);

        if self.use_faucet {
            let faucet_url = if let Some(faucet_url) = self.faucet_url {
                faucet_url
            } else if let Some(url) = CliConfig::load()?.faucet_url {
                Url::parse(&url)
                    .map_err(|err| CliError::UnableToParse("config faucet_url", err.to_string()))?
            } else {
                Url::parse(DEFAULT_FAUCET_URL).map_err(|err| {
                    CliError::UnexpectedError(format!("Failed to parse default faucet URL {}", err))
                })?
            };

            Self::create_account_with_faucet(faucet_url, self.initial_coins, address).await
        } else {
            self.create_account_with_key(address).await
        }
        .map(|_| format!("Account Created at {}", address))
    }

    async fn post_account(
        &self,
        address: AccountAddress,
        sender_key: Ed25519PrivateKey,
        sender_address: AccountAddress,
        sequence_number: u64,
    ) -> CliTypedResult<Response<Transaction>> {
        let client = RestClient::new(reqwest::Url::clone(&self.write_options.rest_options.url()?));
        let transaction_factory = TransactionFactory::new(self.write_options.chain_id().await?)
            .with_gas_unit_price(1)
            .with_max_gas_amount(self.write_options.max_gas);
        let sender_account = &mut LocalAccount::new(sender_address, sender_key, sequence_number);
        let transaction = sender_account.sign_with_transaction_builder(
            transaction_factory
                .payload(aptos_stdlib::encode_create_account_script_function(address)),
        );
        client
            .submit_and_wait(&transaction)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))
    }

    pub async fn create_account_with_faucet(
        faucet_url: reqwest::Url,
        initial_coins: u64,
        address: AccountAddress,
    ) -> CliTypedResult<()> {
        let response = reqwest::Client::new()
            // TODO: Currently, we are just using mint 0 to create an account using the faucet
            // We should make a faucet endpoint for creating an account
            .post(format!(
                "{}mint?amount={}&auth_key={}",
                faucet_url, initial_coins, address
            ))
            .send()
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?;
        if response.status() == 200 {
            Ok(())
        } else {
            Err(CliError::ApiError(format!(
                "Faucet issue: {}",
                response.status()
            )))
        }
    }

    async fn create_account_with_key(self, address: AccountAddress) -> CliTypedResult<()> {
        let client = RestClient::new(reqwest::Url::clone(&self.write_options.rest_options.url()?));
        let sender_private_key = self
            .write_options
            .private_key_options
            .extract_private_key(self.encoding_options.encoding)?;
        let sender_public_key = sender_private_key.public_key();
        let sender_address = account_address_from_public_key(&sender_public_key);
        let sequence_number = client
            .get_account(sender_address)
            .await
            .map_err(|err| CliError::ApiError(err.to_string()))?
            .into_inner()
            .sequence_number;
        self.post_account(address, sender_private_key, sender_address, sequence_number)
            .await?;
        Ok(())
    }
}
