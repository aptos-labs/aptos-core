// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{format_output, BlockArgs, NetworkArgs, UrlArgs};
use aptos_rosetta::types::AccountIdentifier;
use aptos_rosetta::{
    common::native_coin,
    types::{AccountBalanceRequest, AccountBalanceResponse},
};
use aptos_types::account_address::AccountAddress;
use clap::{Parser, Subcommand};

/// Account APIs
///
/// Used for pulling state of an account at a point in time
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
    #[clap(flatten)]
    url_args: UrlArgs,
    #[clap(flatten)]
    block_args: BlockArgs,
    /// Whether to filter the currency to the native coin
    #[clap(long)]
    filter_currency: bool,
    /// Account to list the balance
    #[clap(long, parse(try_from_str=aptos::common::types::load_account_arg))]
    account: AccountAddress,

    #[clap(long)]
    stake_amount: bool,
}

impl AccountBalanceCommand {
    pub async fn execute(self) -> anyhow::Result<AccountBalanceResponse> {
        let account_identifier = if self.stake_amount {
            AccountIdentifier::total_stake_account(self.account)
        } else {
            AccountIdentifier::base_account(self.account)
        };

        let client = self.url_args.client();
        client
            .account_balance(&AccountBalanceRequest {
                network_identifier: self.network_args.network_identifier(),
                account_identifier,
                block_identifier: self.block_args.into(),
                currencies: if self.filter_currency {
                    Some(vec![native_coin()])
                } else {
                    None
                },
            })
            .await
    }
}
