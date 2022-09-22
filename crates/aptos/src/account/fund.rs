// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::time::SystemTime;

use crate::{
    account::create::DEFAULT_FUNDED_COINS,
    common::{
        types::{CliCommand, CliError, CliTypedResult, FaucetOptions, ProfileOptions, RestOptions},
        utils::fund_account,
    },
};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;

/// Command to fund an account with tokens from a faucet
///
/// If the account doesn't exist, it will create it when funding it from the faucet
#[derive(Debug, Parser)]
pub struct FundWithFaucet {
    /// Address to fund
    #[clap(long, parse(try_from_str=crate::common::types::load_account_arg))]
    pub(crate) account: AccountAddress,

    /// Coins to fund when using the faucet
    #[clap(long, default_value_t = DEFAULT_FUNDED_COINS)]
    pub(crate) amount: u64,

    #[clap(flatten)]
    pub(crate) faucet_options: FaucetOptions,
    #[clap(flatten)]
    pub(crate) rest_options: RestOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
}

#[async_trait]
impl CliCommand<String> for FundWithFaucet {
    fn command_name(&self) -> &'static str {
        "FundWithFaucet"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let hashes = fund_account(
            self.faucet_options
                .faucet_url(&self.profile_options.profile)?,
            self.amount,
            self.account,
        )
        .await?;
        let sys_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| CliError::UnexpectedError(e.to_string()))?
            .as_secs()
            + 10;
        let client = self.rest_options.client(&self.profile_options.profile)?;
        for hash in hashes {
            client.wait_for_transaction_by_hash(hash, sys_time).await?;
        }
        return Ok(format!(
            "Added {} Octas to account {}",
            self.amount, self.account
        ));
    }
}
