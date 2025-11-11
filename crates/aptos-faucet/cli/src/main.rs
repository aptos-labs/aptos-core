// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_faucet_core::funder::{
    ApiConnectionConfig, AssetConfig, DEFAULT_ASSET_NAME, FunderTrait, MintAssetConfig, MintFunder, TransactionSubmissionConfig,
};
use std::collections::HashMap;
use aptos_sdk::{
    crypto::ed25519::Ed25519PublicKey,
    types::{
        account_address::AccountAddress, transaction::authenticator::AuthenticationKey,
        LocalAccount,
    },
};
use clap::Parser;
use std::{collections::HashSet, path::PathBuf, str::FromStr};

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

    /// Path to the private key file for minting coins.
    /// To manually generate a keypair, use generate-key:
    /// `cargo run -p generate-keypair -- -o <output_file_path>`
    #[clap(long, default_value = "/opt/aptos/etc/mint.key")]
    pub key_file_path: PathBuf,

    /// The maximum amount of gas in OCTA to spend on a single transaction.
    #[clap(long, default_value_t = 500_000)]
    pub max_gas_amount: u64,
}

impl FaucetCliArgs {
    async fn run(&self) -> Result<()> {
        // Create an AssetConfig to get the key
        let asset_config = AssetConfig::new(
            None,
            self.key_file_path.clone(),
        );

        // Get network root key from the asset config.
        let key = asset_config
            .get_key()
            .context("Failed to build root key")?;

        // Build the account that the MintFunder will use.
        let faucet_account = LocalAccount::new(
            self.mint_account_address.unwrap_or_else(|| {
                AuthenticationKey::ed25519(&Ed25519PublicKey::from(&key)).account_address()
            }),
            key,
            0,
        );

        // Build the txn submission config for the funder.
        let transaction_submission_config = TransactionSubmissionConfig::new(
            None, // maximum_amount
            None, // maximum_amount_with_bypass
            30,   // gas_unit_price_ttl_secs
            None, // gas_unit_price_override
            self.max_gas_amount,
            25,   // transaction_expiration_secs
            30,   // wait_for_outstanding_txns_secs
            true, // wait_for_transactions
        );

        // Create asset configuration for the default asset
        let base_asset_config = AssetConfig::new(
            None,
            self.key_file_path.clone(),
        );
        let mint_asset_config = MintAssetConfig::new(
            base_asset_config,
            self.mint_account_address,
            false, // do_not_delegate is set to false - CLI uses delegation
        );

        // Build assets map with the default asset
        let mut assets = HashMap::new();
        assets.insert(DEFAULT_ASSET_NAME.to_string(), mint_asset_config);

        // Build the MintFunder service.
        let mint_funder = MintFunder::new(
            self.api_connection_args.node_url.clone(),
            self.api_connection_args.api_key.clone(),
            self.api_connection_args.additional_headers.clone(),
            self.api_connection_args.chain_id,
            transaction_submission_config,
            faucet_account,
            assets,
            DEFAULT_ASSET_NAME.to_string(),
            self.amount,
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
            let response = mint_funder
                .fund(Some(self.amount), account, None, false, false)
                .await;
            match response {
                Ok(response) => println!(
                    "SUCCESS: Account: {}, txn hashes: {:?}",
                    account,
                    response
                        .into_iter()
                        .map(|r| r.committed_hash().to_string())
                        .collect::<Vec<_>>()
                ),
                Err(err) => println!("FAILURE: Account: {} Response: {:#}", account, err),
            }
        }

        Ok(())
    }
}

/// Allow 0x to be in front of addresses.
fn process_account_address(str: &str) -> AccountAddress {
    let str = str.trim();
    if let Ok(address) = AccountAddress::from_str(str) {
        address
    } else {
        panic!("Account address is in an invalid format {}", str)
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    FaucetCliArgs::command().debug_assert()
}
