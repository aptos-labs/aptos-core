// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_crypto::{ed25519::Ed25519PrivateKey, ValidCryptoMaterialStringExt};
use aptos_release_builder::{
    initialize_aptos_core_path,
    validate::{DEFAULT_RESOLUTION_TIME, FAST_RESOLUTION_TIME},
};
use aptos_types::{account_address::AccountAddress, chain_id::ChainId};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
pub struct Argument {
    #[clap(subcommand)]
    cmd: Commands,
    #[clap(long)]
    aptos_core_path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    GenerateProposals {
        #[clap(short, long)]
        release_config: PathBuf,
        #[clap(short, long)]
        output_dir: PathBuf,
    },
    WriteDefault {
        #[clap(short, long)]
        output_path: PathBuf,
    },
    ValidateProposals {
        /// Path to the config to be released.
        #[clap(short, long)]
        release_config: PathBuf,
        #[clap(short, long)]
        endpoint: url::Url,
        #[clap(long)]
        framework_git_rev: Option<String>,
        /// Set this value if you want to get the generated proposal at the same time.
        #[clap(long)]
        output_dir: Option<PathBuf>,
        #[clap(subcommand)]
        input_option: InputOptions,
        /// Mint to validator such that it has enough stake to allow fast voting resolution.
        #[clap(long)]
        mint_to_validator: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum InputOptions {
    FromDirectory {
        /// Path to the local testnet folder. If you are running local testnet via cli, it should be `.aptos/testnet`.
        #[clap(short, long)]
        test_dir: PathBuf,
    },
    FromArgs {
        /// Hex encoded string for the root key of the network.
        #[clap(long)]
        root_key: String,
        /// Hex encoded string for the address of a validator node.
        #[clap(long)]
        validator_address: String,
        /// Hex encoded string for the private key of a validator node.
        #[clap(long)]
        validator_key: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Argument::parse();
    initialize_aptos_core_path(args.aptos_core_path.clone());

    // TODO: Being able to parse the release config from a TOML file to generate the proposals.
    match args.cmd {
        Commands::GenerateProposals {
            release_config,
            output_dir,
        } => aptos_release_builder::ReleaseConfig::load_config(release_config.as_path())?
            .generate_release_proposal_scripts(output_dir.as_path()),
        Commands::WriteDefault { output_path } => {
            aptos_release_builder::ReleaseConfig::default().save_config(output_path.as_path())
        },
        Commands::ValidateProposals {
            release_config,
            input_option,
            endpoint,
            framework_git_rev,
            mint_to_validator,
            output_dir,
        } => {
            let config =
                aptos_release_builder::ReleaseConfig::load_config(release_config.as_path())?;

            let root_key_path = aptos_temppath::TempPath::new();
            root_key_path.create_as_file().unwrap();

            let mut network_config = match input_option {
                InputOptions::FromDirectory { test_dir } => {
                    aptos_release_builder::validate::NetworkConfig::new_from_dir(
                        endpoint.clone(),
                        test_dir.as_path(),
                    )?
                },
                InputOptions::FromArgs {
                    root_key,
                    validator_address,
                    validator_key,
                } => {
                    let root_key = Ed25519PrivateKey::from_encoded_string(&root_key)?;
                    let validator_key = Ed25519PrivateKey::from_encoded_string(&validator_key)?;
                    let validator_account = AccountAddress::from_hex(validator_address.as_bytes())?;

                    let mut root_key_path = root_key_path.path().to_path_buf();
                    root_key_path.set_extension("key");

                    std::fs::write(root_key_path.as_path(), bcs::to_bytes(&root_key)?)?;

                    aptos_release_builder::validate::NetworkConfig {
                        root_key_path,
                        validator_account,
                        validator_key,
                        framework_git_rev: None,
                        endpoint: endpoint.clone(),
                    }
                },
            };

            network_config.framework_git_rev = framework_git_rev;

            if mint_to_validator {
                let chain_id = aptos_rest_client::Client::new(endpoint)
                    .get_ledger_information()
                    .await?
                    .inner()
                    .chain_id;

                if chain_id == ChainId::mainnet().id() || chain_id == ChainId::testnet().id() {
                    anyhow::bail!("Mint to mainnet/testnet is not allowed");
                }

                network_config.mint_to_validator().await?;
            }

            network_config
                .set_fast_resolve(FAST_RESOLUTION_TIME)
                .await?;
            aptos_release_builder::validate::validate_config_and_generate_release(
                config,
                network_config.clone(),
                output_dir,
            )
            .await?;
            // Reset resolution time back to normal after resolution
            network_config
                .set_fast_resolve(DEFAULT_RESOLUTION_TIME)
                .await
        },
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Argument::command().debug_assert()
}
