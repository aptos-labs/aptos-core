// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_crypto::{ed25519::Ed25519PrivateKey, ValidCryptoMaterialStringExt};
use aptos_types::account_address::AccountAddress;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
pub struct Argument {
    #[clap(subcommand)]
    cmd: Commands,
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
        #[clap(short, long)]
        release_config: PathBuf,
        #[clap(short, long)]
        endpoint: url::Url,
        #[clap(long)]
        framework_git_rev: Option<String>,
        #[clap(subcommand)]
        input_option: InputOptions,
        #[clap(long)]
        mint_to_validator: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum InputOptions {
    FromDirectory {
        #[clap(short, long)]
        test_dir: PathBuf,
    },
    FromArgs {
        #[clap(long)]
        root_key: String,
        #[clap(long)]
        validator_address: String,
        #[clap(long)]
        validator_key: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Argument::parse();

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
        } => {
            let config =
                aptos_release_builder::ReleaseConfig::load_config(release_config.as_path())?;

            let root_key_path = aptos_temppath::TempPath::new();
            root_key_path.create_as_file().unwrap();

            let mut network_config = match input_option {
                InputOptions::FromDirectory { test_dir } => {
                    aptos_release_builder::validate::NetworkConfig::new_from_dir(
                        endpoint,
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
                        endpoint,
                    }
                },
            };

            network_config.framework_git_rev = framework_git_rev;

            if mint_to_validator {
                network_config.mint_to_validator().await?;
            }

            network_config.set_fast_resolve(30).await?;
            aptos_release_builder::validate::validate_config(config, network_config.clone())
                .await?;
            // Reset resolution time back to normal after resolution
            network_config.set_fast_resolve(43200).await
        },
    }
}
