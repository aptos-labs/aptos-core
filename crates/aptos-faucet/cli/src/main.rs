// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_faucet_core::funder::{ApiConnectionConfig, FunderTrait, MintFunder};
use aptos_sdk::{
    crypto::ed25519::Ed25519PublicKey,
    types::{
        account_address::AccountAddress, transaction::authenticator::AuthenticationKey,
        LocalAccount,
    },
};
use clap::Parser;
use std::{collections::HashSet, str::FromStr, time::Duration};

#[tokio::main]
async fn main() -> Result<()> {
    aptos_logger::Logger::new()
        .level(aptos_logger::Level::Warn)
        .init();
    let args: FaucetCliArgs = FaucetCliArgs::parse();
    args.run().await
}

#[derive(Debug, Parser)]
#[clap(name = "aptos-faucet-cli", author, version)]
pub struct FaucetCliArgs {
    #[clap(flatten)]
    api_connection_args: ApiConnectionConfig,

    /// Amount of coins to mint in OCTA.
    #[clap(long)]
    pub amount: u64,

    /// Addresses of accounts to mint coins to, split by commas.
    #[clap(long)]
    pub accounts: String,

    /// Address of the account to send transactions from. On testnet, for
    /// example, this is a550c18. If not given, we use the account address
    /// corresponding to the given private key. If you forget to set this
    /// while still using a root key, it will fail.
    #[clap(long)]
    pub mint_account_address: Option<AccountAddress>,

    /// The maximum amount of gas in OCTA to spend on a single transaction.
    #[clap(long, default_value_t = 500_000)]
    pub max_gas_amount: u64,
}

impl FaucetCliArgs {
    async fn run(&self) -> Result<()> {
        // Get network root key based on the connection config.
        let key = self
            .api_connection_args
            .get_key()
            .context("Failed to build root key")?;

        // Build the account that the MintFunder will use.
        let faucet_account = LocalAccount::new(
            self.mint_account_address.unwrap_or_else(|| {
                AuthenticationKey::ed25519(&Ed25519PublicKey::from(&key)).derived_address()
            }),
            key,
            0,
        );

        // Build the MintFunder service.
        let mut mint_funder = MintFunder::new(
            faucet_account,
            self.api_connection_args.chain_id,
            self.api_connection_args.node_url.clone(),
            None,
            Duration::from_secs(30),
            None,
            self.max_gas_amount,
            25, // transaction_expiration_secs
            30, // wait_for_outstanding_txns_secs
            true,
        );

        // Create an account that we'll delegate mint functionality to, then use it.
        mint_funder
            .use_delegated_account()
            .await
            .context("Failed to make MintFunder use delegated account")?;

        let accounts: HashSet<AccountAddress> = self
            .accounts
            .trim()
            .split(',')
            .map(process_account_address)
            .collect();

        // Mint coins to each of the accounts.
        for account in accounts {
            let response = mint_funder.fund(Some(self.amount), account, false).await;
            match response {
                Ok(response) => println!(
                    "SUCCESS: Account: {}, txn hashes: {:?}",
                    account.to_hex_literal(),
                    response
                        .into_iter()
                        .map(|r| r.committed_hash().to_hex_literal())
                        .collect::<Vec<_>>()
                ),
                Err(err) => println!(
                    "FAILURE: Account: {} Response: {:#}",
                    account.to_hex_literal(),
                    err
                ),
            }
        }

        Ok(())
    }
}

/// Allow 0x to be in front of addresses.
fn process_account_address(str: &str) -> AccountAddress {
    let str = str.trim();
    if let Ok(address) = AccountAddress::from_hex_literal(str) {
        address
    } else if let Ok(address) = AccountAddress::from_str(str) {
        address
    } else {
        panic!("Account address is in an invalid format {}", str)
    }
}
