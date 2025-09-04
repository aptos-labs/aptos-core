// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::types::FaucetOptions;
use crate::{
    account::key_rotation::lookup_address,
    common::{
        types::{
            account_address_from_public_key, get_mint_site_url, CliCommand, CliConfig, CliError,
            CliTypedResult, ConfigSearchMode, EncodingOptions, HardwareWalletOptions,
            PrivateKeyInputOptions, ProfileConfig, ProfileOptions, PromptOptions, RngArgs,
            DEFAULT_PROFILE,
        },
        utils::{
            explorer_account_link, fund_account, prompt_yes_with_override, read_line,
            strip_private_key_prefix,
        },
    },
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, ValidCryptoMaterialStringExt};
use aptos_ledger;
use async_trait::async_trait;
use clap::Parser;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    str::FromStr,
};

/// 1 APT (might not actually get that much, depending on the faucet)
const NUM_DEFAULT_OCTAS: u64 = 100000000;

/// Tool to initialize current directory for the aptos tool
///
/// Configuration will be pushed into .aptos/config.yaml
#[derive(Debug, Parser)]
pub struct InitTool {
    /// Network to use for default settings
    ///
    /// If custom `rest_url` and `faucet_url` are wanted, use `custom`
    #[clap(long)]
    pub network: Option<Network>,

    /// URL to a fullnode on the network
    #[clap(long)]
    pub rest_url: Option<Url>,

    #[clap(flatten)]
    pub faucet_options: FaucetOptions,

    /// Whether to skip the faucet for a non-faucet endpoint
    #[clap(long)]
    pub skip_faucet: bool,

    /// Whether you want to create a profile from your ledger account
    ///
    /// Make sure that you have your Ledger device connected and unlocked, with the Aptos app installed and opened.
    /// You must also enable "Blind Signing" on your device to sign transactions from the CLI.
    #[clap(long)]
    pub ledger: bool,

    #[clap(flatten)]
    pub(crate) hardware_wallet_options: HardwareWalletOptions,

    #[clap(flatten)]
    pub rng_args: RngArgs,
    #[clap(flatten)]
    pub(crate) private_key_options: PrivateKeyInputOptions,
    #[clap(flatten)]
    pub(crate) profile_options: ProfileOptions,
    #[clap(flatten)]
    pub(crate) prompt_options: PromptOptions,
    #[clap(flatten)]
    pub(crate) encoding_options: EncodingOptions,
}

#[async_trait]
impl CliCommand<()> for InitTool {
    fn command_name(&self) -> &'static str {
        "AptosInit"
    }

    #[allow(clippy::literal_string_with_formatting_args)]
    async fn execute(self) -> CliTypedResult<()> {
        let mut config = if CliConfig::config_exists(ConfigSearchMode::CurrentDir) {
            CliConfig::load(ConfigSearchMode::CurrentDir)?
        } else {
            CliConfig::default()
        };

        let profile_name = self
            .profile_options
            .profile_name()
            .unwrap_or(DEFAULT_PROFILE);

        // Select profile we're using
        let mut profile_config = match config.remove_profile(profile_name) { Some(profile_config) => {
            prompt_yes_with_override(&format!("Aptos already initialized for profile {}, do you want to overwrite the existing config?", profile_name), self.prompt_options)?;
            profile_config
        } _ => {
            ProfileConfig::default()
        }};
        eprintln!("Configuring for profile {}", profile_name);

        // Choose a network
        let network = if let Some(network) = self.network {
            eprintln!("Configuring for network {:?}", network);
            network
        } else {
            eprintln!(
                "Choose network from [devnet, testnet, mainnet, local, custom | defaults to devnet]"
            );
            let input = read_line("network")?;
            let input = input.trim();
            if input.is_empty() {
                eprintln!("No network given, using devnet...");
                Network::Devnet
            } else {
                Network::from_str(input)?
            }
        };

        if network != Network::Custom {
            if self.rest_url.is_some() {
                return Err(CliError::CommandArgumentError(
                    "--rest-url can only be used with --network custom".to_string(),
                ));
            }
            if self.faucet_options.faucet_url.is_some() {
                return Err(CliError::CommandArgumentError(
                    "--faucet-url can only be used with --network custom".to_string(),
                ));
            }
        }

        // Ensure the config contains the network used
        profile_config.network = Some(network);

        // Ensure that there is at least a REST URL set for the network
        match network {
            Network::Mainnet => {
                profile_config.rest_url =
                    Some("https://fullnode.mainnet.aptoslabs.com".to_string());
                profile_config.faucet_url = None;
            },
            Network::Testnet => {
                profile_config.rest_url =
                    Some("https://fullnode.testnet.aptoslabs.com".to_string());
                // The faucet in testnet is only accessible with some kind of bypass.
                // For regular users this can only really mean an auth token. So if
                // there is no auth token set, we don't set the faucet URL. If the user
                // is confident they want to use the testnet faucet without a token
                // they can set it manually with `--network custom` and `--faucet-url`.
                profile_config.faucet_url = None;
            },
            Network::Devnet => {
                profile_config.rest_url = Some("https://fullnode.devnet.aptoslabs.com".to_string());
                profile_config.faucet_url = Some("https://faucet.devnet.aptoslabs.com".to_string());
            },
            Network::Local => {
                profile_config.rest_url = Some("http://localhost:8080".to_string());
                profile_config.faucet_url = Some("http://localhost:8081".to_string());
            },
            Network::Custom => self.custom_network(&mut profile_config)?,
        }

        // Check if any ledger flag is set
        let derivation_path = if let Some(deri_path) =
            self.hardware_wallet_options.extract_derivation_path()?
        {
            Some(deri_path)
        } else if self.ledger {
            // Fetch the top 5 (index 0-4) accounts from Ledger
            let account_map = aptos_ledger::fetch_batch_accounts(Some(0..5))?;
            eprintln!(
                "Please choose an index from the following {} ledger accounts, or choose an arbitrary index that you want to use:",
                account_map.len()
            );

            // Iterate through the accounts and print them out
            for (index, (derivation_path, account)) in account_map.iter().enumerate() {
                eprintln!(
                    "[{}] Derivation path: {} (Address: {})",
                    index, derivation_path, account
                );
            }
            let input_index = read_line("derivation_index")?;
            let input_index = input_index.trim();
            let path = aptos_ledger::DERIVATION_PATH.replace("{index}", input_index);

            // Validate the path
            if !aptos_ledger::validate_derivation_path(&path) {
                return Err(CliError::UnexpectedError(
                    "Invalid index input. Please make sure the input is a valid number index"
                        .to_owned(),
                ));
            }
            Some(path)
        } else {
            None
        };

        // Set the derivation_path to the one user chose
        profile_config.derivation_path.clone_from(&derivation_path);

        // Private key
        let private_key = if self.is_hardware_wallet() {
            // Private key stays in ledger
            None
        } else {
            let ed25519_private_key = match self
                .private_key_options
                .extract_private_key_cli(self.encoding_options.encoding)?
            { Some(key) => {
                eprintln!("Using command line argument for private key");
                key
            } _ => {
                eprintln!("Enter your private key as a hex literal (0x...) [Current: {} | No input: Generate new key (or keep one if present)]", profile_config.private_key.as_ref().map(|_| "Redacted").unwrap_or("None"));
                let input = read_line("Private key")?;
                let input = input.trim();
                if input.is_empty() {
                    if let Some(key) = profile_config.private_key {
                        eprintln!("No key given, keeping existing key...");
                        key
                    } else {
                        eprintln!("No key given, generating key...");
                        self.rng_args
                            .key_generator()?
                            .generate_ed25519_private_key()
                    }
                } else {
                    let stripped = strip_private_key_prefix(&input.to_string())?;
                    Ed25519PrivateKey::from_encoded_string(&stripped).map_err(|err| {
                        CliError::UnableToParse("Ed25519PrivateKey", err.to_string())
                    })?
                }
            }};

            Some(ed25519_private_key)
        };

        // Public key
        let public_key = if self.is_hardware_wallet() {
            let pub_key = match aptos_ledger::get_public_key(
                derivation_path
                    .ok_or_else(|| {
                        CliError::UnexpectedError("Invalid derivation path".to_string())
                    })?
                    .as_str(),
                false,
            ) {
                Ok(pub_key_str) => pub_key_str,
                Err(err) => {
                    return Err(CliError::UnexpectedError(format!(
                        "Unexpected Ledger Error: {:?}",
                        err.to_string()
                    )))
                },
            };
            pub_key
        } else {
            private_key.clone().unwrap().public_key()
        };

        let rest_url = Url::parse(
            profile_config
                .rest_url
                .as_ref()
                .expect("Must have rest client as created above"),
        )
        .map_err(|err| CliError::UnableToParse("rest_url", err.to_string()))?;
        let client = aptos_rest_client::Client::new(rest_url);

        // lookup the address from onchain instead of deriving it
        // if this is the rotated key, deriving it will outputs an incorrect address
        let derived_address = account_address_from_public_key(&public_key);
        let address = lookup_address(&client, derived_address, false).await?;

        profile_config.private_key = private_key;
        profile_config.public_key = Some(public_key);
        profile_config.account = Some(address);

        // Create account if it doesn't exist (and there's a faucet)
        // Check if account exists
        let funded = matches!(client
            .get_account_balance(address, "0x1::AptosCoin::AptosCoin")
            .await, Ok(res) if *res.inner() > 0);

        // If you want to create a private key, but not fund the account, skipping the faucet is still possible
        let maybe_faucet_url = if self.skip_faucet {
            None
        } else {
            profile_config.faucet_url.as_ref()
        };

        if let Some(faucet_url) = maybe_faucet_url {
            if funded {
                eprintln!("Account {} has been already funded onchain", address);
            } else {
                eprintln!(
                    "Account {} is not funded, funding it with {} Octas",
                    address, NUM_DEFAULT_OCTAS
                );
                fund_account(
                    client,
                    Url::parse(faucet_url)
                        .map_err(|err| CliError::UnableToParse("rest_url", err.to_string()))?,
                    self.faucet_options.faucet_auth_token.as_deref(),
                    address,
                    NUM_DEFAULT_OCTAS,
                )
                .await?;
                eprintln!("Account {} funded successfully", address);
            }
        } else if funded {
            eprintln!("Account {} has been already funded onchain", address);
        } else if network == Network::Mainnet || network == Network::Testnet {
            // Do nothing, we print information later.
        } else {
            eprintln!("Account {} has been initialized locally, but you must transfer coins to it to send transactions", address);
        }

        // Ensure the loaded config has profiles setup for a possible empty file
        if config.profiles.is_none() {
            config.profiles = Some(BTreeMap::new());
        }
        config
            .profiles
            .as_mut()
            .expect("Must have profiles, as created above")
            .insert(profile_name.to_string(), profile_config);
        config.save()?;

        let profile_name = self
            .profile_options
            .profile_name()
            .unwrap_or(DEFAULT_PROFILE);

        eprintln!(
            "\n---\nAptos CLI is now set up for account {} as profile {}!\n---\n",
            address, profile_name,
        );

        if !funded {
            match network {
                Network::Mainnet => {
                    eprintln!("The account has not been funded on chain yet, you will need to create and fund the account by transferring funds from another account");
                },
                Network::Testnet => {
                    let mint_site_url = get_mint_site_url(Some(address));
                    eprintln!("The account has not been funded on chain yet. To fund the account and get APT on testnet you must visit {}", mint_site_url);
                    // We don't use `prompt_yes_with_override` here because we only want to
                    // automatically open the minting site if they're in an interactive setting.
                    if !self.prompt_options.assume_yes {
                        eprint!("Press [Enter] to go there now > ");
                        read_line("Confirmation")?;
                        open::that(&mint_site_url).map_err(|err| {
                            CliError::UnexpectedError(format!(
                                "Failed to open minting site: {}",
                                err
                            ))
                        })?;
                    }
                },
                _ => {},
            }
        } else {
            eprintln!(
                "See the account here: {}",
                explorer_account_link(address, Some(network))
            );
        }

        Ok(())
    }
}

impl InitTool {
    /// Custom network created, which requires a REST URL
    fn custom_network(&self, profile_config: &mut ProfileConfig) -> CliTypedResult<()> {
        // Rest Endpoint
        let rest_url = if let Some(ref rest_url) = self.rest_url {
            eprintln!("Using command line argument for rest URL {}", rest_url);
            Some(rest_url.to_string())
        } else {
            let current = profile_config.rest_url.as_deref();
            eprintln!(
                    "Enter your rest endpoint [Current: {} | No input: Exit (or keep the existing if present)]",
                    current.unwrap_or("None"),
                );
            let input = read_line("Rest endpoint")?;
            let input = input.trim();
            if input.is_empty() {
                if let Some(current) = current {
                    eprintln!("No rest url given, keeping the existing url...");
                    Some(current.to_string())
                } else {
                    eprintln!("No rest url given, exiting...");
                    return Err(CliError::AbortedError);
                }
            } else {
                Some(
                    reqwest::Url::parse(input)
                        .map_err(|err| CliError::UnableToParse("Rest Endpoint", err.to_string()))?
                        .to_string(),
                )
            }
        };
        profile_config.rest_url = rest_url;

        // Faucet Endpoint
        let faucet_url = if self.skip_faucet {
            eprintln!("Not configuring a faucet because --skip-faucet was provided");
            None
        } else if let Some(ref faucet_url) = self.faucet_options.faucet_url {
            eprintln!("Using command line argument for faucet URL {}", faucet_url);
            Some(faucet_url.to_string())
        } else {
            let current = profile_config.faucet_url.as_deref();
            eprintln!(
                    "Enter your faucet endpoint [Current: {} | No input: Skip (or keep the existing one if present) | 'skip' to not use a faucet]",
                    current
                        .unwrap_or("None"),
                );
            let input = read_line("Faucet endpoint")?;
            let input = input.trim();
            if input.is_empty() {
                if let Some(current) = current {
                    eprintln!("No faucet url given, keeping the existing url...");
                    Some(current.to_string())
                } else {
                    eprintln!("No faucet url given, skipping faucet...");
                    None
                }
            } else if input.to_lowercase() == "skip" {
                eprintln!("Skipping faucet...");
                None
            } else {
                Some(
                    reqwest::Url::parse(input)
                        .map_err(|err| CliError::UnableToParse("Faucet Endpoint", err.to_string()))?
                        .to_string(),
                )
            }
        };
        profile_config.faucet_url = faucet_url;
        Ok(())
    }

    fn is_hardware_wallet(&self) -> bool {
        self.hardware_wallet_options.is_hardware_wallet() || self.ledger
    }
}

/// A simplified list of all networks supported by the CLI
///
/// Any command using this, will be simpler to setup as profiles
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
    Local,
    Custom,
}

impl Display for Network {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Network::Mainnet => "mainnet",
            Network::Testnet => "testnet",
            Network::Devnet => "devnet",
            Network::Local => "local",
            Network::Custom => "custom",
        })
    }
}

impl FromStr for Network {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().trim() {
            "mainnet" => Self::Mainnet,
            "testnet" => Self::Testnet,
            "devnet" => Self::Devnet,
            "local" => Self::Local,
            "custom" => Self::Custom,
            str => {
                return Err(CliError::CommandArgumentError(format!(
                    "Invalid network {}.  Must be one of [devnet, testnet, mainnet, local, custom]",
                    str
                )));
            },
        })
    }
}

impl Default for Network {
    fn default() -> Self {
        Self::Devnet
    }
}
