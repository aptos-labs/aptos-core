// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::file::{
    create_dir_if_not_exist, current_dir, home_dir, read_from_file, write_to_user_only_file,
};
use crate::parse::{from_yaml, to_yaml};
use crate::types::{CliError, CliTypedResult};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_types::account_address::AccountAddress;
use clap::ArgEnum;
use clap::Parser;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use std::fmt::Formatter;
use std::path::PathBuf;
use std::str::FromStr;

pub const DEFAULT_REST_URL: &str = "https://fullnode.devnet.aptoslabs.com";
pub const CONFIG_FOLDER: &str = ".aptos";
const CONFIG_FILE: &str = "config.yaml";
const LEGACY_CONFIG_FILE: &str = "config.yml";
const GLOBAL_CONFIG_FILE: &str = "global_config.yaml";
const GLOBAL: &str = "global";
const WORKSPACE: &str = "workspace";

/// A global configuration for global settings related to a user
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GlobalConfig {
    /// Whether to be using Global or Workspace mode
    pub config_type: ConfigType,
}

impl GlobalConfig {
    pub fn load() -> CliTypedResult<Self> {
        let path = global_folder()?.join(GLOBAL_CONFIG_FILE);
        if path.exists() {
            from_yaml(&String::from_utf8(read_from_file(path.as_path())?)?)
        } else {
            // If we don't have a config, let's load the default
            Ok(GlobalConfig::default())
        }
    }

    /// Get the config location based on the type
    pub fn get_config_location(&self) -> CliTypedResult<PathBuf> {
        match self.config_type {
            ConfigType::Global => global_folder(),
            ConfigType::Workspace => Ok(current_dir()?.join(CONFIG_FOLDER)),
        }
    }

    pub fn save(&self) -> CliTypedResult<()> {
        let global_folder = global_folder()?;
        create_dir_if_not_exist(global_folder.as_path())?;

        write_to_user_only_file(
            global_folder.join(GLOBAL_CONFIG_FILE).as_path(),
            "Global Config",
            &to_yaml(&self)?.into_bytes(),
        )
    }
}

fn global_folder() -> CliTypedResult<PathBuf> {
    Ok(home_dir()?.join(CONFIG_FOLDER))
}

/// A configuration for where to place and use the config
///
/// Workspace allows for multiple configs based on location, where
/// Global allows for one config for every part of the code
#[derive(Debug, Copy, Clone, Serialize, Deserialize, ArgEnum)]
pub enum ConfigType {
    /// Per system user configuration put in `<HOME>/.aptos`
    Global,
    /// Per directory configuration put in `<CURRENT_DIR>/.aptos`
    Workspace,
}

impl Default for ConfigType {
    fn default() -> Self {
        // TODO: When we version up, we can change this to global
        Self::Workspace
    }
}

impl std::fmt::Display for ConfigType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ConfigType::Global => GLOBAL,
            ConfigType::Workspace => WORKSPACE,
        })
    }
}

impl FromStr for ConfigType {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            GLOBAL => Ok(Self::Global),
            WORKSPACE => Ok(Self::Workspace),
            _ => Err(CliError::CommandArgumentError(
                "Invalid config type, must be one of [global, workspace]".to_string(),
            )),
        }
    }
}

/// An individual profile
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Private key for commands.
    pub private_key: Option<Ed25519PrivateKey>,
    /// Public key for commands
    pub public_key: Option<Ed25519PublicKey>,
    /// Account for commands
    pub account: Option<AccountAddress>,
    /// URL for the Aptos rest endpoint
    pub rest_url: Option<String>,
    /// URL for the Faucet endpoint (if applicable)
    pub faucet_url: Option<String>,
}

/// Config saved to `.aptos/config.yaml`
#[derive(Debug, Serialize, Deserialize)]
pub struct CliConfig {
    /// Map of profile configs
    pub profiles: Option<HashMap<String, ProfileConfig>>,
}

impl Default for CliConfig {
    fn default() -> Self {
        CliConfig {
            profiles: Some(HashMap::new()),
        }
    }
}

impl CliConfig {
    /// Checks if the config exists in the current working directory
    pub fn config_exists() -> bool {
        if let Ok(folder) = Self::aptos_folder() {
            let config_file = folder.join(CONFIG_FILE);
            let old_config_file = folder.join(LEGACY_CONFIG_FILE);
            config_file.exists() || old_config_file.exists()
        } else {
            false
        }
    }

    /// Loads the config from the current working directory
    pub fn load() -> CliTypedResult<Self> {
        let folder = Self::aptos_folder()?;

        let config_file = folder.join(CONFIG_FILE);
        let old_config_file = folder.join(LEGACY_CONFIG_FILE);
        if config_file.exists() {
            from_yaml(
                &String::from_utf8(read_from_file(config_file.as_path())?)
                    .map_err(CliError::from)?,
            )
        } else if old_config_file.exists() {
            from_yaml(
                &String::from_utf8(read_from_file(old_config_file.as_path())?)
                    .map_err(CliError::from)?,
            )
        } else {
            Err(CliError::ConfigNotFoundError(format!(
                "{}",
                config_file.display()
            )))
        }
    }

    pub fn load_profile(profile: &str) -> CliTypedResult<Option<ProfileConfig>> {
        let mut config = Self::load()?;
        Ok(config.remove_profile(profile))
    }

    pub fn remove_profile(&mut self, profile: &str) -> Option<ProfileConfig> {
        if let Some(ref mut profiles) = self.profiles {
            profiles.remove(&profile.to_string())
        } else {
            None
        }
    }

    /// Saves the config to ./.aptos/config.yaml
    pub fn save(&self) -> CliTypedResult<()> {
        let aptos_folder = Self::aptos_folder()?;

        // Create if it doesn't exist
        create_dir_if_not_exist(aptos_folder.as_path())?;

        // Save over previous config file
        let config_file = aptos_folder.join(CONFIG_FILE);
        let config_bytes = serde_yaml::to_string(&self).map_err(|err| {
            CliError::UnexpectedError(format!("Failed to serialize config {}", err))
        })?;
        write_to_user_only_file(&config_file, CONFIG_FILE, config_bytes.as_bytes())?;

        // As a cleanup, delete the old if it exists
        let legacy_config_file = aptos_folder.join(LEGACY_CONFIG_FILE);
        if legacy_config_file.exists() {
            eprintln!("Removing legacy config file {}", LEGACY_CONFIG_FILE);
            let _ = std::fs::remove_file(legacy_config_file);
        }
        Ok(())
    }

    /// Finds the current directory's .aptos folder
    fn aptos_folder() -> CliTypedResult<PathBuf> {
        let global_config = GlobalConfig::load()?;
        global_config.get_config_location()
    }
}

#[derive(Debug, Parser)]
pub struct ProfileOptions {
    /// Profile to use from config
    #[clap(long, default_value = "default")]
    pub profile: String,
}

impl ProfileOptions {
    pub fn account_address(&self) -> CliTypedResult<AccountAddress> {
        if let Some(profile) = CliConfig::load_profile(&self.profile)? {
            if let Some(account) = profile.account {
                return Ok(account);
            }
        }

        Err(CliError::ConfigNotFoundError(self.profile.clone()))
    }
}

impl Default for ProfileOptions {
    fn default() -> Self {
        Self {
            profile: "default".to_string(),
        }
    }
}
