// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Command-line entry point for `aptos_indexer_transaction_generator::ci_helper`.

use anyhow::{Context, Result};
use aptos_indexer_transaction_generator::ci_helper;
use clap::{Parser, Subcommand};
use std::{
    env, fs,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[command(about = "Trusted CI helper for indexer transaction fixtures")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Validate imported_transactions.yaml and the sibling move_fixtures folder.
    ValidateConfig {
        #[arg(long)]
        config: PathBuf,
    },
    /// Validate the configuration and write a copy with the protected API keys.
    MaterializeConfig {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    /// Compare generated transactions with the checked-in baseline.
    Compare {
        #[arg(long)]
        baseline_dir: PathBuf,
        #[arg(long)]
        generated_dir: PathBuf,
        #[arg(long)]
        github_output: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Command::ValidateConfig { config } => {
            ci_helper::validate_config_file(&config)?;
            println!("Validated transaction generator configuration.");
        },
        Command::MaterializeConfig { input, output } => {
            let config = ci_helper::validate_config_file(&input)?;
            let materialized = ci_helper::materialize_config(config, |network| {
                env::var(network.secret_env()).ok()
            })?;
            ci_helper::write_private_file(&output, &materialized)?;
            println!("Materialized protected transaction generator configuration.");
        },
        Command::Compare {
            baseline_dir,
            generated_dir,
            github_output,
        } => {
            let comparison = ci_helper::compare(&baseline_dir, &generated_dir)?;
            write_file(&github_output, &comparison.github_outputs())?;
        },
    }
    Ok(())
}

fn write_file(path: &Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}
