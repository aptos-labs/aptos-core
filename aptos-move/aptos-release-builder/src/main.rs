// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use aptos_crypto::{ed25519::Ed25519PrivateKey, ValidCryptoMaterialStringExt};
use aptos_release_builder::{
    components::fetch_config,
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
    /// Generate sets of governance proposals based on the release_config file passed in
    GenerateProposals {
        #[clap(short, long)]
        release_config: PathBuf,
        #[clap(short, long)]
        output_dir: PathBuf,
    },
    /// Generate sets of governance proposals with default release config.
    WriteDefault {
        #[clap(short, long)]
        output_path: PathBuf,
    },
    /// Execute governance proposals generated from a given release config.
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
    /// Print out current values of on chain configs.
    PrintConfigs {
        /// Url endpoint for the desired network. e.g: https://fullnode.mainnet.aptoslabs.com/v1.
        #[clap(short, long)]
        endpoint: url::Url,
        /// Whether to print out the full gas schedule.
        #[clap(short, long)]
        print_gas_schedule: bool,
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
async fn main() {
    let args = Argument::parse();
    initialize_aptos_core_path(args.aptos_core_path.clone());

    // TODO: Being able to parse the release config from a TOML file to generate the proposals.
    match args.cmd {
        Commands::GenerateProposals {
            release_config,
            output_dir,
        } => aptos_release_builder::ReleaseConfig::load_config(release_config.as_path())
            .with_context(|| "Failed to load release config".to_string())
            .unwrap()
            .generate_release_proposal_scripts(output_dir.as_path())
            .with_context(|| "Failed to generate release proposal scripts".to_string())
            .unwrap(),
        Commands::WriteDefault { output_path } => aptos_release_builder::ReleaseConfig::default()
            .save_config(output_path.as_path())
            .unwrap(),
        Commands::ValidateProposals {
            release_config,
            input_option,
            endpoint,
            framework_git_rev,
            mint_to_validator,
            output_dir,
        } => {
            let config =
                aptos_release_builder::ReleaseConfig::load_config(release_config.as_path())
                    .unwrap();

            let root_key_path = aptos_temppath::TempPath::new();
            root_key_path.create_as_file().unwrap();

            let mut network_config = match input_option {
                InputOptions::FromDirectory { test_dir } => {
                    aptos_release_builder::validate::NetworkConfig::new_from_dir(
                        endpoint.clone(),
                        test_dir.as_path(),
                    )
                    .unwrap()
                },
                InputOptions::FromArgs {
                    root_key,
                    validator_address,
                    validator_key,
                } => {
                    let root_key = Ed25519PrivateKey::from_encoded_string(&root_key).unwrap();
                    let validator_key =
                        Ed25519PrivateKey::from_encoded_string(&validator_key).unwrap();
                    let validator_account =
                        AccountAddress::from_hex(validator_address.as_bytes()).unwrap();

                    let mut root_key_path = root_key_path.path().to_path_buf();
                    root_key_path.set_extension("key");

                    std::fs::write(root_key_path.as_path(), bcs::to_bytes(&root_key).unwrap())
                        .unwrap();

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
                    .await
                    .unwrap()
                    .inner()
                    .chain_id;

                if chain_id == ChainId::mainnet().id() || chain_id == ChainId::testnet().id() {
                    panic!("Mint to mainnet/testnet is not allowed");
                }

                network_config.mint_to_validator().await.unwrap();
            }

            network_config
                .set_fast_resolve(FAST_RESOLUTION_TIME)
                .await
                .unwrap();
            aptos_release_builder::validate::validate_config_and_generate_release(
                config,
                network_config.clone(),
                output_dir,
            )
            .await
            .unwrap();
            // Reset resolution time back to normal after resolution
            network_config
                .set_fast_resolve(DEFAULT_RESOLUTION_TIME)
                .await
                .unwrap()
        },
        Commands::PrintConfigs {
            endpoint,
            print_gas_schedule,
        } => {
            use aptos_types::on_chain_config::*;

            let client = aptos_rest_client::Client::new(endpoint);

            macro_rules! print_configs {
                ($($type:ty), *) => {
                    $(
                        println!("{}", std::any::type_name::<$type>());
                        println!("{}", serde_yaml::to_string(&fetch_config::<$type>(&client).unwrap()).unwrap());
                    )*
                }
            }

            print_configs!(OnChainConsensusConfig, OnChainExecutionConfig, Version);

            if print_gas_schedule {
                print_configs!(GasScheduleV2, StorageGasSchedule);
            }

            // Print Activated Features
            let features = fetch_config::<Features>(&client).unwrap();
            println!(
                "Features\n{}",
                serde_yaml::to_string(
                    &aptos_release_builder::components::feature_flags::Features::from(&features)
                )
                .unwrap()
            );
        },
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Argument::command().debug_assert()
}
