// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{CliCommand, CliTypedResult, FaucetOptions, TransactionOptions},
    utils::fund_account,
};
use aptos_transaction_builder::aptos_stdlib;
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;

pub const DEFAULT_FUNDED_COINS: u64 = 10000;

/// Command to create a new account on-chain
///
#[derive(Debug, Parser)]
pub struct CreateAccount {
    #[clap(flatten)]
    pub(crate) txn_options: TransactionOptions,
    /// Address to create account for
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) account: AccountAddress,
    /// Flag for using faucet to create the account
    #[clap(long)]
    pub(crate) use_faucet: bool,
    #[clap(flatten)]
    pub(crate) faucet_options: FaucetOptions,
    /// Initial coins to fund when using the faucet
    #[clap(long, default_value_t = DEFAULT_FUNDED_COINS)]
    pub(crate) initial_coins: u64,
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
                    .faucet_url(&self.txn_options.profile_options.profile)?,
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
        self.txn_options
            .submit_transaction(aptos_stdlib::encode_account_create_account(address))
            .await?;
        Ok(())
    }
}
