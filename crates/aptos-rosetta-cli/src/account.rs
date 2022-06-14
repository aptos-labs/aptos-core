// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{format_output, NetworkArgs, UrlArgs};
use aptos_rosetta::types::AccountBalanceResponse;
use aptos_types::account_address::AccountAddress;
use clap::{Parser, Subcommand};

/// Account APIs
///
/// [API Spec](https://www.rosetta-api.org/docs/AccountApi.html)
#[derive(Debug, Subcommand)]
pub enum AccountCommand {
    Balance(AccountBalanceCommand),
}

impl AccountCommand {
    pub async fn execute(self) -> anyhow::Result<String> {
        match self {
            AccountCommand::Balance(inner) => format_output(inner.execute().await),
        }
    }
}

/// Retrieve the balance for an account
///
/// [API Spec](https://www.rosetta-api.org/docs/AccountApi.html#accountbalance)
#[derive(Debug, Parser)]
pub struct AccountBalanceCommand {
    #[clap(flatten)]
    network_args: NetworkArgs,
    /// Account to list the balance
    #[clap(long)]
    account: AccountAddress,
    #[clap(flatten)]
    url_args: UrlArgs,
}

impl AccountBalanceCommand {
    pub async fn execute(self) -> anyhow::Result<AccountBalanceResponse> {
        let client = self.url_args.client();
        client
            .account_balance_simple(self.account, self.network_args.chain_id)
            .await
    }
}
