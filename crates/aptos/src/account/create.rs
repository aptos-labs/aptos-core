// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    init::DEFAULT_FAUCET_URL,
    types::{
        CliCommand, CliConfig, CliError, CliTypedResult, EncodingOptions, ProfileOptions,
        WriteTransactionOptions,
    },
    utils::submit_transaction,
};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;
use reqwest::Url;

/// Command to create a new account on-chain
///
#[derive(Debug, Parser)]
pub struct CreateAccount {
    #[clap(flatten)]
    encoding_options: EncodingOptions,
    #[clap(flatten)]
    write_options: WriteTransactionOptions,
    #[clap(flatten)]
    profile_options: ProfileOptions,
    /// Address to create account for
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    account: AccountAddress,
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

#[async_trait]
impl CliCommand<String> for CreateAccount {
    fn command_name(&self) -> &'static str {
        "CreateAccount"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let address = self.account;
        if self.use_faucet {
            let faucet_url = if let Some(faucet_url) = self.faucet_url {
                faucet_url
            } else if let Some(Some(url)) = CliConfig::load_profile(&self.profile_options.profile)?
                .map(|profile| profile.faucet_url)
            {
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
}

impl CreateAccount {
    pub async fn create_account_with_faucet(
        faucet_url: Url,
        initial_coins: u64,
        address: AccountAddress,
    ) -> CliTypedResult<()> {
        let response = reqwest::Client::new()
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
        let sender_key = self.write_options.private_key_options.extract_private_key(
            self.encoding_options.encoding,
            &self.profile_options.profile,
        )?;

        submit_transaction(
            self.write_options
                .rest_options
                .url(&self.profile_options.profile)?,
            self.write_options
                .chain_id(&self.profile_options.profile)
                .await?,
            sender_key,
            aptos_stdlib::encode_account_create_account(address),
            self.write_options.max_gas,
        )
        .await?;
        Ok(())
    }
}
