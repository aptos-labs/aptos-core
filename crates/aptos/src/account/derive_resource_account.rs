// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::common::types::{CliCommand, CliTypedResult};
use aptos_move_cli::ResourceAccountSeed;
use aptos_types::account_address::{create_resource_address, AccountAddress};
use async_trait::async_trait;
use clap::Parser;

/// Derive the address for a resource account
///
/// This will not create a resource account, but instead give the deterministic address given
/// a source address and seed.
#[derive(Debug, Parser)]
pub struct DeriveResourceAccount {
    /// Address of the creator's account
    #[clap(long, alias = "account", value_parser = crate::common::types::load_account_arg)]
    pub(crate) address: AccountAddress,

    #[clap(flatten)]
    pub(crate) seed_args: ResourceAccountSeed,
}

#[async_trait]
impl CliCommand<AccountAddress> for DeriveResourceAccount {
    fn command_name(&self) -> &'static str {
        "DeriveResourceAccountAddress"
    }

    async fn execute(self) -> CliTypedResult<AccountAddress> {
        let seed = self.seed_args.seed()?;
        Ok(create_resource_address(self.address, &seed))
    }
}
