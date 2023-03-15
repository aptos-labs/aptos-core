// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliError, CliTypedResult};
use aptos_types::account_address::{create_resource_address, AccountAddress};
use async_trait::async_trait;
use clap::Parser;

/// A generic interface for allowing for different types of seed phrase inputs
///
/// The easiest to use is `string_seed` as it will match directly with the b"string" notation in Move.
#[derive(Debug, Parser)]
pub struct ResourceAccountSeed {
    /// Resource account seed - String BCS encoded
    ///
    /// Legacy functionality from previous tooling, this will be renamed.
    ///
    /// Seed used in generation of the AccountId of the resource account
    /// The seed will be converted to bytes using `BCS`
    #[clap(long, group = "resource_account_seed")]
    pub(crate) seed: Option<String>,

    /// Resource account seed - String UTF-8 encoded (preferred)
    ///
    /// Seed used in generation of the AccountId of the resource account
    /// The seed will be converted to bytes directly with UTF-8 encoding
    #[clap(long, group = "resource_account_seed")]
    pub(crate) string_seed: Option<String>,

    /// Hex seed - Bytes hex encoded
    ///
    /// Seed used in generation of the AccountId of the resource account
    /// The seed will directly be converted to associated bytes
    #[clap(long, group = "resource_account_seed")]
    pub(crate) hex_seed: Option<Vec<u8>>,
}

impl ResourceAccountSeed {
    pub fn seed(self) -> CliTypedResult<Vec<u8>> {
        match (self.seed, self.string_seed, self.hex_seed) {
            (Some(ref seed), None, None) => Ok(bcs::to_bytes(seed)?),
            (None, Some(string_seed), None) => Ok(string_seed.as_bytes().to_vec()),
            (None, None, Some(binary_seed)) => Ok(binary_seed),
            _ => Err(CliError::CommandArgumentError("Must have exactly one of the following seed arguments: [\"--seed\", \"--string_seed\", \"--hex_seed\"]".to_string()))
        }
    }
}

/// Derive the address for a resource account
///
/// This will not create a resource account, but instead give the deterministic address given
/// a source address and seed.
#[derive(Debug, Parser)]
pub struct DeriveResourceAccount {
    /// Address of the creator's account
    #[clap(long, alias = "account", parse(try_from_str=crate::common::types::load_account_arg))]
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
