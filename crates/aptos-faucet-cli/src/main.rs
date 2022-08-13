// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos::common::types::EncodingType;
use aptos_config::keys::ConfigKey;
use aptos_crypto::ed25519::Ed25519PrivateKey;
use aptos_faucet::{mint, mint::MintParams, Service};
use aptos_sdk::types::{
    account_address::AccountAddress, account_config::aptos_test_root_address, chain_id::ChainId,
    LocalAccount,
};
use clap::Parser;
use std::{collections::HashSet, path::PathBuf, str::FromStr};
use url::Url;

#[tokio::main]
async fn main() {
    aptos_logger::Logger::new().init();
    let args: FaucetCliArgs = FaucetCliArgs::parse();
    args.execute().await
}

#[derive(Debug, Parser)]
#[clap(name = "aptos-faucet-cli", author, version)]
pub struct FaucetCliArgs {
    /// Aptos fullnode/validator server URL
    #[clap(long, default_value = "https://fullnode.devnet.aptoslabs.com/")]
    pub server_url: Url,
    /// Path to the private key for creating test account and minting coins.
    /// To keep Testnet simple, we used one private key for aptos root account
    /// To manually generate a keypair, use generate-key:
    /// `cargo run -p generate-keypair -- -o <output_file_path>`
    #[clap(long, default_value = "/opt/aptos/etc/mint.key", parse(from_os_str))]
    pub mint_key_file_path: PathBuf,
    /// Ed25519PrivateKey for minting coins
    #[clap(long, parse(try_from_str = ConfigKey::from_encoded_string))]
    pub mint_key: Option<ConfigKey<Ed25519PrivateKey>>,
    /// Address of the account to send transactions from.
    /// On Testnet, for example, this is a550c18.
    /// If not present, the mint key's address is used
    #[clap(long, parse(try_from_str = AccountAddress::from_hex_literal))]
    pub mint_account_address: Option<AccountAddress>,
    /// Chain ID of the network this client is connecting to.
    /// For mainnet: "MAINNET" or 1, testnet: "TESTNET" or 2, devnet: "DEVNET" or 3,
    /// local swarm: "TESTING" or 4
    /// Note: Chain ID of 0 is not allowed; Use number if chain id is not predefined.
    #[clap(long, default_value = "3")]
    pub chain_id: ChainId,
    /// Amount of coins to mint
    #[clap(long)]
    pub amount: u64,
    /// Addresses of accounts to mint coins to, split by commas
    #[clap(long, group = "account-group")]
    pub accounts: Option<String>,
    /// File of addresses of account to mint coins to.  Formatted in YAML
    #[clap(long, group = "account-group", parse(from_os_str))]
    pub account_file: Option<PathBuf>,
}

impl FaucetCliArgs {
    async fn execute(self) {
        let mint_account_address = self
            .mint_account_address
            .unwrap_or_else(aptos_test_root_address);
        let mint_key = if let Some(ref key) = self.mint_key {
            key.private_key()
        } else {
            EncodingType::BCS
                .load_key::<Ed25519PrivateKey>("mint key", self.mint_key_file_path.as_path())
                .unwrap()
        };
        let faucet_account = LocalAccount::new(mint_account_address, mint_key, 0);
        let service = Service::new(self.server_url, self.chain_id, faucet_account, None);

        let accounts: HashSet<AccountAddress> = if let Some(accounts) = self.accounts {
            accounts
                .trim()
                .split(',')
                .map(process_account_address)
                .collect()
        } else if let Some(path) = self.account_file {
            let strings: Vec<String> =
                serde_yaml::from_str(&std::fs::read_to_string(path.as_path()).unwrap()).unwrap();
            strings
                .into_iter()
                .map(|str| process_account_address(&str))
                .collect()
        } else {
            panic!("Either --accounts or --account-file must be specified");
        };

        // Iterate through accounts to mint the tokens
        for account in accounts {
            let response = mint::process(
                &service,
                MintParams {
                    amount: self.amount,
                    auth_key: None,
                    address: Some(account.to_hex_literal()),
                    pub_key: None,
                    return_txns: None,
                },
            )
            .await;
            match response {
                Ok(response) => println!(
                    "SUCCESS: Account: {} Response: {:?}",
                    account.to_hex_literal(),
                    response
                ),
                Err(response) => println!(
                    "FAILURE: Account: {} Response: {:?}",
                    account.to_hex_literal(),
                    response
                ),
            }
        }
    }
}

/// Allow 0x to be in front of addresses
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
