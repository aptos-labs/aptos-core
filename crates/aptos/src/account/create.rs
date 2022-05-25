// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{
        CliCommand, CliTypedResult, EncodingOptions, FaucetOptions, ProfileOptions,
        WriteTransactionOptions,
    },
    utils::{fund_account, submit_transaction},
};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;

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
    #[clap(flatten)]
    faucet_options: FaucetOptions,
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
            fund_account(
                self.faucet_options
                    .faucet_url(&self.profile_options.profile)?,
                self.initial_coins,
                self.account,
            )
            .await
        } else {
            self.create_account_with_key(address).await
        }
        .map(|_| format!("Account Created at {}", address))
    }
}

impl CreateAccount {
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
