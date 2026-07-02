// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    account::create::DEFAULT_FUNDED_COINS,
    common::types::{CliCommand, CliTypedResult, FaucetOptions, ProfileOptions, RestOptions},
};
use aptos_types::account_address::AccountAddress;
use async_trait::async_trait;
use clap::Parser;

/// Fund an account with tokens from a faucet
///
/// This will create an account if it doesn't exist with the faucet.  This is mostly useful
/// for local development and devnet.
#[derive(Debug, Default, Parser)]
#[clap(after_help = "Examples:
  # Fund the current profile's account with the default amount
  $ aptos account fund-with-faucet

  # Fund a specific account with 1 APT (100000000 Octas)
  $ aptos account fund-with-faucet --account 0xc0ffee --amount 100000000

  # Fund using an explicit faucet URL (e.g. a local testnet)
  $ aptos account fund-with-faucet --account 0xc0ffee --faucet-url http://localhost:8081")]
pub struct FundWithFaucet {
    /// Address to fund
    ///
    /// If the account wasn't previously created, it will be created when being funded
    #[clap(long, value_parser = crate::common::types::load_account_arg)]
    pub account: Option<AccountAddress>,

    /// Number of Octas to fund the account from the faucet
    ///
    /// The amount added to the account may be limited by the faucet, and may be less
    /// than the amount requested.
    #[clap(long, default_value_t = DEFAULT_FUNDED_COINS)]
    pub amount: u64,

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
        let address = if let Some(account) = self.account {
            account
        } else {
            self.profile_options.account_address()?
        };
        let client = self.rest_options.client(&self.profile_options)?;
        self.faucet_options
            .fund_account(client, &self.profile_options, self.amount, address)
            .await?;
        return Ok(format!(
            "Added {} Octas to account {}",
            self.amount, address
        ));
    }
}
