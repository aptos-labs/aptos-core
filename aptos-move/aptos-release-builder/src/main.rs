// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
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
        test_dir: PathBuf,
        #[clap(short, long)]
        endpoint: url::Url,
        #[clap(long)]
        framework_git_rev: Option<String>,
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
            test_dir,
            endpoint,
            framework_git_rev,
        } => {
            let config =
                aptos_release_builder::ReleaseConfig::load_config(release_config.as_path())?;

            let mut network_config = aptos_release_builder::validate::NetworkConfig::new_from_dir(
                endpoint,
                test_dir.as_path(),
            )?;

            network_config.framework_git_rev = framework_git_rev;

            aptos_release_builder::validate::validate_config(config, network_config).await
        },
    }
}
