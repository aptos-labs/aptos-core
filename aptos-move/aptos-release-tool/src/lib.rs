// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! `aptos-release-tool` manages on-chain governance proposals — framework
//! releases and ad-hoc changes alike — around a single, self-contained
//! *governance bundle*.
//!
//! It can generate a bundle from a config, verify the bundle's internal integrity,
//! simulate its governance proposal against a network, and verify on-chain state
//! after deployment.

pub mod bundle;
pub mod commands;
pub mod config;
pub mod network;
pub mod release;
pub mod summary;
pub mod table;

use crate::network::NetworkSelection;
use anyhow::Result;
use aptos_types::account_address::AccountAddress;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[clap(author, version, about)]
pub struct Argument {
    #[clap(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a complete release bundle from a release config.
    GenerateBundle {
        /// Path to the release config (e.g. framework-release.yaml).
        #[clap(short, long)]
        release_config: PathBuf,

        /// Bundle directory to create. Must not already exist.
        #[clap(short, long)]
        bundle: PathBuf,
    },

    /// Validate that a bundle is internally self-consistent.
    VerifyBundle {
        /// Path to the bundle directory.
        #[clap(short, long)]
        bundle: PathBuf,

        /// Also require that all summary sign-off checkboxes are ticked.
        #[clap(long)]
        require_signoff: bool,
    },

    /// Simulate a bundle's governance proposals against a network.
    Simulate {
        /// Path to the bundle directory.
        #[clap(short, long)]
        bundle: PathBuf,

        /// Network to simulate against.
        ///
        /// Possible values: devnet, testnet, mainnet, <url to rest endpoint>
        #[clap(long)]
        network: NetworkSelection,

        /// Enable the gas profiler.
        #[clap(long, default_value_t = false)]
        profile_gas: bool,

        /// API key for node API ratelimiting.
        /// May also be set via the NODE_API_KEY environment variable.
        #[clap(long, env)]
        node_api_key: Option<String>,
    },

    /// Execute a bundle's governance proposal on a test network. Never mainnet.
    ///
    /// Runs the full preflight (verify-bundle with sign-off, execution-hash
    /// reproducibility check, simulation) and then the test-network governance
    /// ceremony: shrink the voting period (root), propose and vote
    /// (validator), execute every step, restore the voting period.
    DeployTestnet {
        /// Path to the bundle directory.
        #[clap(short, long)]
        bundle: PathBuf,

        /// Network to deploy to. Mainnet is refused.
        ///
        /// Possible values: devnet, testnet, <url to rest endpoint>
        #[clap(long)]
        network: NetworkSelection,

        /// Hex-encoded root (core resources) private key. Signs only the
        /// voting-period changes, which are test-network-only operations.
        #[clap(long, env = "ROOT_KEY", hide_env_values = true)]
        root_key: String,

        /// The validator's stake pool address, used to propose and vote.
        #[clap(long, env = "VALIDATOR_ADDRESS")]
        validator_address: AccountAddress,

        /// Hex-encoded private key of the validator's voter account.
        #[clap(long, env = "VALIDATOR_KEY", hide_env_values = true)]
        validator_key: String,

        /// API key for node API ratelimiting.
        /// May also be set via the NODE_API_KEY environment variable.
        #[clap(long, env)]
        node_api_key: Option<String>,

        /// Metadata URL recorded on-chain with the proposal. Informational
        /// only; nothing in this flow fetches it. The default is a dummy URL
        /// on the RFC 2606 reserved `.invalid` TLD, which is guaranteed to
        /// never resolve.
        #[clap(long, default_value = "https://dummy.invalid/proposal-metadata.json")]
        metadata_url: String,

        /// Mint gas funds to the validator first (root-signed). For throwaway
        /// networks (local swarms, forge); refused on testnet.
        #[clap(long)]
        mint_to_validator: bool,

        /// Do not require the bundle's sign-off checkboxes to be ticked.
        #[clap(long)]
        skip_signoff: bool,

        /// Skip the pre-deployment simulation.
        #[clap(long)]
        skip_simulation: bool,

        /// Run every preflight check, then stop before submitting anything.
        #[clap(long)]
        dry_run: bool,
    },

    /// Verify a deployed framework release on-chain against a bundle.
    VerifyFrameworkDeployment {
        /// Path to the bundle directory.
        #[clap(short, long)]
        bundle: PathBuf,

        /// Network to check.
        ///
        /// Possible values: devnet, testnet, mainnet, <url to rest endpoint>
        #[clap(long)]
        network: NetworkSelection,

        /// API key for node API ratelimiting (`Authorization: Bearer <key>`).
        /// May also be set via the NODE_API_KEY environment variable.
        #[clap(long, env)]
        node_api_key: Option<String>,
    },
}

/// Repo root, derived from this crate's location at build time.
fn aptos_core_path() -> PathBuf {
    let mut path = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
    path.pop(); // aptos-move
    path.pop(); // repo root
    path.canonicalize().unwrap_or(path)
}

/// Compute the repo root and register it with `aptos-release-builder`, whose
/// framework script generation reads it as a global to locate the framework
/// sources. Returns the path for callers that also need it directly (e.g. to
/// read source provenance).
pub fn init_core_path() -> PathBuf {
    let core_path = aptos_core_path();
    aptos_release_builder::initialize_aptos_core_path(Some(core_path.clone()));
    core_path
}

/// Dispatch a parsed CLI invocation.
pub async fn run(args: Argument) -> Result<()> {
    let core_path = init_core_path();

    match args.cmd {
        Commands::GenerateBundle {
            release_config,
            bundle,
        } => commands::generate::run(&release_config, &bundle, &core_path).await,
        Commands::VerifyBundle {
            bundle,
            require_signoff,
        } => commands::verify::run(&bundle, require_signoff),
        Commands::DeployTestnet {
            bundle,
            network,
            root_key,
            validator_address,
            validator_key,
            node_api_key,
            metadata_url,
            mint_to_validator,
            skip_signoff,
            skip_simulation,
            dry_run,
        } => {
            commands::deploy::run(
                &bundle,
                network.to_url()?,
                commands::deploy::Signers {
                    root_key,
                    validator_address,
                    validator_key,
                },
                node_api_key,
                &metadata_url,
                &core_path,
                mint_to_validator,
                skip_signoff,
                skip_simulation,
                dry_run,
            )
            .await
        },
        Commands::Simulate {
            bundle,
            network,
            profile_gas,
            node_api_key,
        } => commands::simulate::run(&bundle, &network, profile_gas, node_api_key).await,
        Commands::VerifyFrameworkDeployment {
            bundle,
            network,
            node_api_key,
        } => commands::verify_framework_deployment::run(&bundle, &network, node_api_key).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn verify_tool() {
        Argument::command().debug_assert()
    }
}
