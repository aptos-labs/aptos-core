// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::ConfigSearchMode;
use crate::common::{
    types::{
        account_address_from_public_key, CliCommand, CliConfig, CliError, CliTypedResult,
        EncodingOptions, PrivateKeyInputOptions, ProfileConfig, ProfileOptions, PromptOptions,
        RngArgs,
    },
    utils::{fund_account, prompt_yes_with_override, read_line},
};
use aptos_crypto::{ed25519::Ed25519PrivateKey, PrivateKey, ValidCryptoMaterialStringExt};
use async_trait::async_trait;
use clap::Parser;
use reqwest::Url;
use std::collections::BTreeMap;

pub const DEFAULT_REST_URL: &str = "https://fullnode.devnet.aptoslabs.com/v1";
pub const DEFAULT_FAUCET_URL: &str = "https://faucet.devnet.aptoslabs.com";
const NUM_DEFAULT_COINS: u64 = 10000;

/// Tool to initialize current directory for the aptos tool
///
/// Configuration will be pushed into .aptos/config.yaml
#[derive(Debug, Parser)]
pub struct InitTool {
    /// URL to a fullnode on the network
    #[clap(long)]
    pub rest_url: Option<Url>,

    /// URL for the Faucet endpoint
    #[clap(long)]
    pub faucet_url: Option<Url>,

    /// Whether to skip the faucet for a non-faucet endpoint
    #[clap(long)]
    pub skip_faucet: bool,

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

    async fn execute(self) -> CliTypedResult<()> {
        let mut config = if CliConfig::config_exists(ConfigSearchMode::CurrentDir) {
            CliConfig::load(ConfigSearchMode::CurrentDir)?
        } else {
            CliConfig::default()
        };

        // Select profile we're using
        let mut profile_config = if let Some(profile_config) =
            config.remove_profile(&self.profile_options.profile)
        {
            prompt_yes_with_override(&format!("Aptos already initialized for profile {}, do you want to overwrite the existing config?", self.profile_options.profile), self.prompt_options)?;
            profile_config
        } else {
            ProfileConfig::default()
        };

        eprintln!("Configuring for profile {}", self.profile_options.profile);

        // Rest Endpoint
        let rest_url = if let Some(rest_url) = self.rest_url {
            eprintln!("Using command line argument for rest URL {}", rest_url);
            rest_url
        } else {
            eprintln!(
                "Enter your rest endpoint [Current: {} | No input: {}]",
                profile_config
                    .rest_url
                    .unwrap_or_else(|| "None".to_string()),
                DEFAULT_REST_URL
            );
            let input = read_line("Rest endpoint")?;
            let input = input.trim();
            if input.is_empty() {
                eprintln!("No rest url given, using {}...", DEFAULT_REST_URL);
                reqwest::Url::parse(DEFAULT_REST_URL).map_err(|err| {
                    CliError::UnexpectedError(format!("Failed to parse default rest URL {}", err))
                })?
            } else {
                reqwest::Url::parse(input)
                    .map_err(|err| CliError::UnableToParse("Rest Endpoint", err.to_string()))?
            }
        };
        profile_config.rest_url = Some(rest_url.to_string());

        // Faucet Endpoint
        let faucet_url = if self.skip_faucet {
            eprintln!("Not configuring a faucet because --skip-faucet was provided");
            None
        } else if let Some(faucet_url) = self.faucet_url {
            eprintln!("Using command line argument for faucet URL {}", faucet_url);
            Some(faucet_url)
        } else {
            eprintln!(
                "Enter your faucet endpoint [Current: {} | No input: {} | 'skip' to not use a faucet]",
                profile_config
                    .faucet_url
                    .unwrap_or_else(|| "None".to_string()),
                DEFAULT_FAUCET_URL
            );
            let input = read_line("Faucet endpoint")?;
            let input = input.trim();
            if input.is_empty() {
                eprintln!("No faucet url given, using {}...", DEFAULT_FAUCET_URL);
                Some(reqwest::Url::parse(DEFAULT_FAUCET_URL).map_err(|err| {
                    CliError::UnexpectedError(format!("Failed to parse default faucet URL {}", err))
                })?)
            } else if input.to_lowercase() == "skip" {
                eprintln!("Skipping faucet");
                None
            } else {
                Some(
                    reqwest::Url::parse(input).map_err(|err| {
                        CliError::UnableToParse("Faucet Endpoint", err.to_string())
                    })?,
                )
            }
        };
        profile_config.faucet_url = faucet_url.as_ref().map(|inner| inner.to_string());

        // Private key
        let private_key = if let Some(private_key) = self
            .private_key_options
            .extract_private_key_cli(self.encoding_options.encoding)?
        {
            eprintln!("Using command line argument for private key");
            private_key
        } else {
            eprintln!("Enter your private key as a hex literal (0x...) [Current: {} | No input: Generate new key (or keep one if present)]", profile_config.private_key.as_ref().map(|_| "Redacted").unwrap_or("None"));
            let input = read_line("Private key")?;
            let input = input.trim();
            if input.is_empty() {
                if let Some(private_key) = profile_config.private_key {
                    eprintln!("No key given, keeping existing key...");
                    private_key
                } else {
                    eprintln!("No key given, generating key...");
                    self.rng_args
                        .key_generator()?
                        .generate_ed25519_private_key()
                }
            } else {
                Ed25519PrivateKey::from_encoded_string(input)
                    .map_err(|err| CliError::UnableToParse("Ed25519PrivateKey", err.to_string()))?
            }
        };
        let public_key = private_key.public_key();
        let address = account_address_from_public_key(&public_key);
        profile_config.private_key = Some(private_key);
        profile_config.public_key = Some(public_key);
        profile_config.account = Some(address);

        // Create account if it doesn't exist (and there's a faucet)
        let client = aptos_rest_client::Client::new(rest_url);
        if let Some(faucet_url) = faucet_url {
            if client.get_account(address).await.is_err() {
                eprintln!(
                    "Account {} doesn't exist, creating it and funding it with {} Octas",
                    address, NUM_DEFAULT_COINS
                );
                fund_account(faucet_url, NUM_DEFAULT_COINS, address).await?;
            }
        }

        // Ensure the loaded config has profiles setup for a possible empty file
        if config.profiles.is_none() {
            config.profiles = Some(BTreeMap::new());
        }
        config
            .profiles
            .as_mut()
            .unwrap()
            .insert(self.profile_options.profile, profile_config);
        config.save()?;
        eprintln!("Aptos is now set up for account {}!  Run `aptos help` for more information about commands", address);
        Ok(())
    }
}
