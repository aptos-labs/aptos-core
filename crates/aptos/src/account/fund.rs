// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::{
    types::{CliCommand, CliTypedResult, FaucetOptions, ProfileOptions},
    utils::fund_account,
};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;

/// Command to fund an account with tokens from a faucet
///
#[derive(Debug, Parser)]
pub struct FundAccount {
    #[clap(flatten)]
    profile_options: ProfileOptions,
    /// Address to create account for
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    account: AccountAddress,
    #[clap(flatten)]
    faucet_options: FaucetOptions,
    /// Coins to fund when using the faucet
    #[clap(long, default_value = "10000")]
    num_coins: u64,
}

#[async_trait]
impl CliCommand<String> for FundAccount {
    fn command_name(&self) -> &'static str {
        "FundAccount"
    }

    async fn execute(self) -> CliTypedResult<String> {
        fund_account(
            self.faucet_options
                .faucet_url(&self.profile_options.profile)?,
            self.num_coins,
            self.account,
        )
        .await
        .map(|_| format!("Added {} coins to account {}", self.num_coins, self.account))
    }
}
