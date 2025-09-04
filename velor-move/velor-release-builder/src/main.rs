// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{bail, Context};
use velor_crypto::{ed25519::Ed25519PrivateKey, ValidCryptoMaterialStringExt};
use velor_framework::natives::code::PackageRegistry;
use velor_gas_schedule::LATEST_GAS_FEATURE_VERSION;
use velor_release_builder::{
    components::fetch_config,
    initialize_velor_core_path,
    simulate::simulate_all_proposals,
    validate::{DEFAULT_RESOLUTION_TIME, FAST_RESOLUTION_TIME},
};
use velor_rest_client::{VelorBaseUrl, Client};
use velor_types::{
    account_address::AccountAddress,
    chain_id::ChainId,
    jwks::{ObservedJWKs, SupportedOIDCProviders},
};
use clap::{Parser, Subcommand};
use std::{path::PathBuf, str::FromStr};
use url::Url;

#[derive(Parser)]
pub struct Argument {
    #[clap(subcommand)]
    cmd: Commands,
    #[clap(long)]
    velor_core_path: Option<PathBuf>,
}

// TODO(vgao1996): unify with `ReplayNetworkSelection` in the `velor` crate.
#[derive(Clone, Debug)]
pub enum NetworkSelection {
    Mainnet,
    Testnet,
    Devnet,
    RestEndpoint(String),
}

impl FromStr for NetworkSelection {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, anyhow::Error> {
        Ok(match s {
            "mainnet" => Self::Mainnet,
            "testnet" => Self::Testnet,
            "devnet" => Self::Devnet,
            _ => Self::RestEndpoint(s.to_owned()),
        })
    }
}

impl NetworkSelection {
    fn to_url(&self) -> anyhow::Result<Url> {
        use NetworkSelection::*;

        let s = match &self {
            Mainnet => "https://fullnode.mainnet.velorlabs.com",
            Testnet => "https://fullnode.testnet.velorlabs.com",
            Devnet => "https://fullnode.devnet.velorlabs.com",
            RestEndpoint(url) => url,
        };

        Ok(Url::parse(s)?)
    }
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Generate sets of governance proposals based on the release_config file passed in
    GenerateProposals {
        /// Path to the release config.
        #[clap(short, long)]
        release_config: PathBuf,

        /// Output directory to store the generated artifacts.
        #[clap(short, long)]
        output_dir: PathBuf,

        /// If set, simulate the governance proposals after generation.
        #[clap(long)]
        simulate: Option<NetworkSelection>,

        /// Set this flag to enable the gas profiler.
        /// Can only be used in combination with `--simulate`.
        #[clap(long)]
        profile_gas: Option<bool>,

        /// Key to use for ratelimiting purposes with the node API. This value will be used
        /// as `Authorization: Bearer <key>`. You may also set this with the NODE_API_KEY
        /// environment variable.
        #[clap(long, env)]
        node_api_key: Option<String>,
    },
    /// Simulate a multi-step proposal on the specified network, using its current states.
    /// The simulation will execute the governance scripts, as if the proposal is already
    /// approved.
    Simulate {
        /// Directory that may contain one or more proposals at any level
        /// within its sub-directory hierarchy.
        #[clap(short, long)]
        path: PathBuf,

        /// The network to simulate on.
        ///
        /// Possible values: devnet, testnet, mainnet, <url to rest endpoint>
        #[clap(long)]
        network: NetworkSelection,

        /// Set this flag to enable the gas profiler
        #[clap(long, default_value_t = false)]
        profile_gas: bool,

        /// Key to use for ratelimiting purposes with the node API. This value will be used
        /// as `Authorization: Bearer <key>`. You may also set this with the NODE_API_KEY
        /// environment variable.
        #[clap(long, env)]
        node_api_key: Option<String>,
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

        /// Key to use for ratelimiting purposes with the node API. This value will be used
        /// as `Authorization: Bearer <key>`. You may also set this with the NODE_API_KEY
        /// environment variable.
        #[clap(long, env)]
        node_api_key: Option<String>,
    },
    /// Generate a gas schedule using the current values and store it to a file.
    GenerateGasSchedule {
        /// The version of the gas schedule to generate.
        #[clap(short, long)]
        version: Option<u64>,

        /// Path of the output file.
        #[clap(short, long)]
        output_path: Option<PathBuf>,
    },
    /// Print out current values of on chain configs.
    PrintConfigs {
        /// Url endpoint for the desired network. e.g: https://fullnode.mainnet.velorlabs.com/v1.
        #[clap(short, long)]
        endpoint: url::Url,
        /// Whether to print out the full gas schedule.
        #[clap(short, long)]
        print_gas_schedule: bool,
        /// Key to use for ratelimiting purposes with the node API. This value will be used
        /// as `Authorization: Bearer <key>`. You may also set this with the NODE_API_KEY
        /// environment variable.
        #[clap(long, env)]
        node_api_key: Option<String>,
    },
    /// Print out package metadata.
    /// Usage: --endpoint '<URL>'
    /// --package-address <ADDRESS> --package-name <PACKAGE_NAME> [--print-json]
    PrintPackageMetadata {
        /// Url endpoint for the desired network. e.g: https://fullnode.mainnet.velorlabs.com/v1.
        #[clap(short, long)]
        endpoint: url::Url,
        /// The address under which the package is published
        #[clap(long)]
        package_address: String,
        /// The name of the package
        #[clap(long)]
        package_name: String,
        /// Whether to print the original data in json
        #[clap(long)]
        print_json: bool,
        /// Key to use for ratelimiting purposes with the node API. This value will be used
        /// as `Authorization: Bearer <key>`. You may also set this with the NODE_API_KEY
        /// environment variable.
        #[clap(long, env)]
        node_api_key: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum InputOptions {
    FromDirectory {
        /// Path to the localnet folder. If you are running localnet via cli, it should be `.velor/testnet`.
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
async fn main() -> anyhow::Result<()> {
    let args = Argument::parse();
    initialize_velor_core_path(args.velor_core_path.clone());

    // TODO: Being able to parse the release config from a TOML file to generate the proposals.
    match args.cmd {
        Commands::GenerateProposals {
            release_config,
            output_dir,
            simulate,
            profile_gas,
            node_api_key,
        } => {
            velor_release_builder::ReleaseConfig::load_config(release_config.as_path())
                .with_context(|| "Failed to load release config".to_string())?
                .generate_release_proposal_scripts(output_dir.as_path())
                .await
                .with_context(|| "Failed to generate release proposal scripts".to_string())?;

            match simulate {
                Some(network) => {
                    let profile_gas = profile_gas.unwrap_or(false);
                    let remote_endpoint = network.to_url()?;
                    simulate_all_proposals(
                        remote_endpoint,
                        output_dir.as_path(),
                        profile_gas,
                        node_api_key,
                    )
                    .await?;
                },
                None => {
                    if profile_gas.is_some() {
                        bail!("--profile-gas can only be set in combination with --simulate")
                    }
                },
            }

            Ok(())
        },
        Commands::Simulate {
            network,
            path,
            profile_gas,
            node_api_key,
        } => {
            simulate_all_proposals(network.to_url()?, &path, profile_gas, node_api_key).await?;
            Ok(())
        },
        Commands::WriteDefault { output_path } => {
            velor_release_builder::ReleaseConfig::default().save_config(output_path.as_path())
        },
        Commands::ValidateProposals {
            release_config,
            input_option,
            endpoint,
            framework_git_rev,
            mint_to_validator,
            output_dir,
            node_api_key,
        } => {
            let config =
                velor_release_builder::ReleaseConfig::load_config(release_config.as_path())?;

            let root_key_path = velor_temppath::TempPath::new();
            root_key_path.create_as_file()?;

            let mut network_config = match input_option {
                InputOptions::FromDirectory { test_dir } => {
                    velor_release_builder::validate::NetworkConfig::new_from_dir(
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

                    velor_release_builder::validate::NetworkConfig {
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
                let mut client = Client::builder(VelorBaseUrl::Custom(endpoint));
                if let Some(api_key) = node_api_key.as_ref() {
                    client = client.api_key(api_key)?;
                }
                let chain_id = client
                    .build()
                    .get_ledger_information()
                    .await?
                    .inner()
                    .chain_id;

                if chain_id == ChainId::mainnet().id() || chain_id == ChainId::testnet().id() {
                    panic!("Mint to mainnet/testnet is not allowed");
                }

                network_config
                    .mint_to_validator(node_api_key.clone())
                    .await?;
            }

            network_config
                .set_fast_resolve(FAST_RESOLUTION_TIME)
                .await?;
            velor_release_builder::validate::validate_config_and_generate_release(
                config,
                network_config.clone(),
                output_dir,
                node_api_key.clone(),
            )
            .await?;
            // Reset resolution time back to normal after resolution
            network_config
                .set_fast_resolve(DEFAULT_RESOLUTION_TIME)
                .await?;
            Ok(())
        },
        Commands::GenerateGasSchedule {
            version,
            output_path,
        } => {
            let version = version.unwrap_or(LATEST_GAS_FEATURE_VERSION);
            let output_path =
                output_path.unwrap_or_else(|| PathBuf::from_str("gas_schedule.json").unwrap());

            let gas_schedule = velor_gas_schedule_updator::current_gas_schedule(version);
            let json = serde_json::to_string_pretty(&gas_schedule)?;

            std::fs::write(&output_path, json)?;
            println!("Gas scheduled saved to {}.", output_path.display());

            Ok(())
        },
        Commands::PrintConfigs {
            endpoint,
            print_gas_schedule,
            node_api_key,
        } => {
            use velor_types::on_chain_config::*;

            let mut client = Client::builder(VelorBaseUrl::Custom(endpoint));
            if let Some(api_key) = node_api_key {
                client = client.api_key(&api_key)?;
            }
            let client = client.build();

            macro_rules! print_configs {
                ($($type:ty), *) => {
                    $(
                        println!("{}", std::any::type_name::<$type>());
                        println!("{}", serde_yaml::to_string(&fetch_config::<$type>(&client)?)?);
                    )*
                }
            }

            print_configs!(OnChainConsensusConfig, OnChainExecutionConfig, VelorVersion);

            if print_gas_schedule {
                print_configs!(GasScheduleV2, StorageGasSchedule);
            }

            // Print Activated Features
            let features = fetch_config::<Features>(&client)?;
            println!(
                "Features\n{}",
                serde_yaml::to_string(
                    &velor_release_builder::components::feature_flags::Features::from(&features)
                )?
            );

            let oidc_providers = fetch_config::<SupportedOIDCProviders>(&client);
            let observed_jwks = fetch_config::<ObservedJWKs>(&client);
            let jwk_consensus_config = fetch_config::<OnChainJWKConsensusConfig>(&client);
            let randomness_config = fetch_config::<RandomnessConfigMoveStruct>(&client)
                .and_then(OnChainRandomnessConfig::try_from);
            println!();
            println!("SupportedOIDCProviders");
            println!("{:?}", oidc_providers);
            println!();
            println!("ObservedJWKs");
            println!("{:?}", observed_jwks);
            println!();
            println!("JWKConsensusConfig");
            println!("{:?}", jwk_consensus_config);
            println!();
            println!("RandomnessConfig");
            println!("{:?}", randomness_config);
            Ok(())
        },
        Commands::PrintPackageMetadata {
            endpoint,
            package_address,
            package_name,
            print_json,
            node_api_key,
        } => {
            let mut client = Client::builder(VelorBaseUrl::Custom(endpoint));
            if let Some(api_key) = node_api_key {
                client = client.api_key(&api_key)?;
            }
            let client = client.build();
            let address = AccountAddress::from_str_strict(&package_address)?;
            let packages = client
                .get_account_resource_bcs::<PackageRegistry>(address, "0x1::code::PackageRegistry")
                .await?;
            for package in packages.into_inner().packages {
                if package.name == package_name {
                    if print_json {
                        println!("{}", serde_json::to_string(&package).unwrap());
                    } else {
                        println!("{}", package);
                    }
                    break;
                }
            }
            Ok(())
        },
    }
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Argument::command().debug_assert()
}
