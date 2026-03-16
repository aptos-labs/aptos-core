// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! CLI configuration management: profiles, network selection, and persistent settings.
//!
//! Handles loading and saving `CliConfig` profiles (stored in `.aptos/config.yaml`),
//! including network endpoints, keys, and account addresses. Supports optional
//! password-based encryption for sensitive fields.

use crate::{
    create_dir_if_not_exist, current_dir,
    encryption::{get_password, DerivedKey, EncryptionConfig},
    raw_config::{profile_config_to_raw, RawCliConfig},
    read_from_file,
    types::{APTOS_FOLDER_GIT_IGNORE, CONFIG_FOLDER, DEFAULT_PROFILE, GIT_IGNORE},
    utils::{
        deserialize_address_str, deserialize_material_with_prefix, serialize_material_with_prefix,
    },
    write_to_user_only_file, CliError, CliTypedResult,
};
use aptos_crypto::ed25519::{Ed25519PrivateKey, Ed25519PublicKey};
use aptos_types::account_address::AccountAddress;
use clap::ValueEnum;
use serde::{Deserialize as DeserializeTrait, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    path::PathBuf,
    str::FromStr,
};

// ── Network ──

/// A simplified list of all networks supported by the CLI
///
/// Any command using this, will be simpler to setup as profiles
#[derive(Copy, Clone, Debug, Default, Serialize, DeserializeTrait, Eq, PartialEq)]
pub enum Network {
    Mainnet,
    Testnet,
    #[default]
    Devnet,
    Local,
    Custom,
}

impl Network {
    /// Returns the default REST API URL for well-known networks.
    pub fn default_rest_url(&self) -> Option<&'static str> {
        match self {
            Network::Mainnet => Some("https://api.mainnet.aptoslabs.com"),
            Network::Testnet => Some("https://api.testnet.aptoslabs.com"),
            Network::Devnet => Some("https://api.devnet.aptoslabs.com"),
            Network::Local => Some("http://localhost:8080"),
            Network::Custom => None,
        }
    }

    /// Returns the default faucet URL for networks that have one.
    pub fn default_faucet_url(&self) -> Option<&'static str> {
        match self {
            Network::Devnet => Some("https://faucet.devnet.aptoslabs.com"),
            Network::Local => Some("http://localhost:8081"),
            _ => None,
        }
    }
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

// ── ProfileConfig ──

/// An individual profile
#[derive(Debug, Serialize, DeserializeTrait)]
pub struct ProfileConfig {
    /// Profile schema version (managed by the raw config layer, not serialized directly).
    #[serde(skip, default = "crate::raw_config::default_profile_version")]
    pub version: u32,
    /// Name of network being used, if setup from aptos init
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    /// Private key for commands.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        serialize_with = "serialize_material_with_prefix",
        deserialize_with = "deserialize_material_with_prefix"
    )]
    pub private_key: Option<Ed25519PrivateKey>,
    /// Public key for commands
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_material_with_prefix",
        deserialize_with = "deserialize_material_with_prefix"
    )]
    pub public_key: Option<Ed25519PublicKey>,
    /// Account for commands
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_address_str"
    )]
    pub account: Option<AccountAddress>,
    /// URL for the Aptos rest endpoint
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest_url: Option<String>,
    /// URL for the Faucet endpoint (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_url: Option<String>,
    /// Derivation path index of the account on ledger
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derivation_path: Option<String>,
    /// API key for the node REST endpoint (encrypted when encryption is enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_api_key: Option<String>,
    /// Auth token for the faucet endpoint (encrypted when encryption is enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_auth_token: Option<String>,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            version: crate::raw_config::CURRENT_PROFILE_VERSION,
            network: None,
            private_key: None,
            public_key: None,
            account: None,
            rest_url: None,
            faucet_url: None,
            derivation_path: None,
            node_api_key: None,
            faucet_auth_token: None,
        }
    }
}

// ── ProfileSummary ──

/// ProfileConfig but without the private parts
#[derive(Debug, Serialize)]
pub struct ProfileSummary {
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<Network>,
    pub has_private_key: bool,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_material_with_prefix",
        deserialize_with = "deserialize_material_with_prefix"
    )]
    pub public_key: Option<Ed25519PublicKey>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_address_str"
    )]
    pub account: Option<AccountAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rest_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faucet_url: Option<String>,
    pub has_node_api_key: bool,
    pub has_faucet_auth_token: bool,
    pub encrypted: bool,
}

impl From<&ProfileConfig> for ProfileSummary {
    fn from(config: &ProfileConfig) -> Self {
        ProfileSummary {
            version: config.version,
            network: config.network,
            has_private_key: config.private_key.is_some(),
            public_key: config.public_key.clone(),
            account: config.account,
            rest_url: config.rest_url.clone(),
            faucet_url: config.faucet_url.clone(),
            has_node_api_key: config.node_api_key.is_some(),
            has_faucet_auth_token: config.faucet_auth_token.is_some(),
            encrypted: false, // Will be set by the caller when applicable
        }
    }
}

// ── ConfigSearchMode ──

#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum ConfigSearchMode {
    CurrentDir,
    CurrentDirAndParents,
}

// ── CliConfig ──

const CONFIG_FILE: &str = "config.yaml";
const LEGACY_CONFIG_FILE: &str = "config.yml";

/// Config saved to `.aptos/config.yaml`
#[derive(Debug, Serialize, DeserializeTrait)]
pub struct CliConfig {
    /// Encryption metadata (present when config fields are encrypted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<EncryptionConfig>,
    /// Map of profile configs
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profiles: Option<BTreeMap<String, ProfileConfig>>,
}

impl Default for CliConfig {
    fn default() -> Self {
        CliConfig {
            encryption: None,
            profiles: Some(BTreeMap::new()),
        }
    }
}

impl CliConfig {
    /// Checks if the config exists in the current working directory
    pub fn config_exists(mode: ConfigSearchMode) -> bool {
        if let Ok(folder) = Self::aptos_folder(mode) {
            let config_file = folder.join(CONFIG_FILE);
            let old_config_file = folder.join(LEGACY_CONFIG_FILE);
            config_file.exists() || old_config_file.exists()
        } else {
            false
        }
    }

    /// Two-phase load: read YAML as raw strings, detect encryption, decrypt if needed,
    /// then parse into typed ProfileConfig. All downstream code sees typed profiles.
    pub fn load(mode: ConfigSearchMode) -> CliTypedResult<Self> {
        let folder = Self::aptos_folder(mode)?;
        let config_file = folder.join(CONFIG_FILE);
        let old_config_file = folder.join(LEGACY_CONFIG_FILE);

        let yaml_str = if config_file.exists() {
            String::from_utf8(read_from_file(config_file.as_path())?).map_err(CliError::from)?
        } else if old_config_file.exists() {
            String::from_utf8(read_from_file(old_config_file.as_path())?).map_err(CliError::from)?
        } else {
            return Err(CliError::ConfigNotFoundError(format!(
                "{}",
                config_file.display()
            )));
        };

        // Phase 1: Deserialize to raw config (all string values).
        let raw: RawCliConfig = serde_yaml::from_str(&yaml_str)
            .map_err(|e| CliError::UnexpectedError(e.to_string()))?;

        // Phase 2: If encryption section is present and fields are encrypted, derive key.
        let has_encrypted = raw.has_any_encrypted_fields();
        let derived_key = if has_encrypted {
            let enc_config = raw.encryption.as_ref().ok_or_else(|| {
                CliError::EncryptionError(
                    "Config contains encrypted fields but no encryption section".to_string(),
                )
            })?;
            let password = get_password(enc_config, &folder)?;
            let key = DerivedKey::derive(&password, enc_config)?;
            if !key.verify_key_check(enc_config) {
                return Err(CliError::WrongPassword);
            }
            Some(key)
        } else {
            None
        };

        // Phase 3: Convert raw profiles to typed ProfileConfig.
        let profiles = raw
            .profiles
            .map(|raw_profiles| {
                raw_profiles
                    .into_iter()
                    .map(|(name, raw_profile)| {
                        let typed = raw_profile.into_profile_config(derived_key.as_ref())?;
                        Ok((name, typed))
                    })
                    .collect::<CliTypedResult<BTreeMap<String, ProfileConfig>>>()
            })
            .transpose()?;

        Ok(CliConfig {
            encryption: raw.encryption,
            profiles,
        })
    }

    /// Load config without decrypting sensitive fields.
    ///
    /// Encrypted fields become `None` in the resulting `ProfileConfig`.
    /// Use this for commands that only need public config data (rest_url,
    /// network, account, public_key) and don't need private_key, node_api_key,
    /// or faucet_auth_token.
    pub fn load_public(mode: ConfigSearchMode) -> CliTypedResult<Self> {
        let folder = Self::aptos_folder(mode)?;
        let config_file = folder.join(CONFIG_FILE);
        let old_config_file = folder.join(LEGACY_CONFIG_FILE);

        let yaml_str = if config_file.exists() {
            String::from_utf8(read_from_file(config_file.as_path())?).map_err(CliError::from)?
        } else if old_config_file.exists() {
            String::from_utf8(read_from_file(old_config_file.as_path())?).map_err(CliError::from)?
        } else {
            return Err(CliError::ConfigNotFoundError(format!(
                "{}",
                config_file.display()
            )));
        };

        let raw: RawCliConfig = serde_yaml::from_str(&yaml_str)
            .map_err(|e| CliError::UnexpectedError(e.to_string()))?;

        // Detect corrupted config: encrypted fields without encryption section
        if raw.has_any_encrypted_fields() && raw.encryption.is_none() {
            return Err(CliError::EncryptionError(
                "Config contains encrypted fields but no encryption section".to_string(),
            ));
        }

        // Skip password/KDF — pass None as key so encrypted fields become None.
        let profiles = raw
            .profiles
            .map(|raw_profiles| {
                raw_profiles
                    .into_iter()
                    .map(|(name, raw_profile)| {
                        let typed = raw_profile.into_profile_config(None)?;
                        Ok((name, typed))
                    })
                    .collect::<CliTypedResult<BTreeMap<String, ProfileConfig>>>()
            })
            .transpose()?;

        Ok(CliConfig {
            encryption: raw.encryption,
            profiles,
        })
    }

    /// Load a single profile without decrypting sensitive fields.
    ///
    /// Encrypted fields become `None`. Use for commands that only need
    /// public config data.
    pub fn load_profile_public(
        profile: Option<&str>,
        mode: ConfigSearchMode,
    ) -> CliTypedResult<Option<ProfileConfig>> {
        let mut config = Self::load_public(mode)?;

        if let Some(profile) = profile {
            if let Some(account_profile) = config.remove_profile(profile) {
                Ok(Some(account_profile))
            } else {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} not found",
                    profile
                )))
            }
        } else {
            Ok(config.remove_profile(DEFAULT_PROFILE))
        }
    }

    /// Check whether a specific field in a profile is encrypted in the raw YAML.
    pub fn is_profile_field_encrypted(
        profile: Option<&str>,
        mode: ConfigSearchMode,
        field_name: &str,
    ) -> bool {
        let Ok(folder) = Self::aptos_folder(mode) else {
            return false;
        };
        let config_file = folder.join(CONFIG_FILE);
        let old_config_file = folder.join(LEGACY_CONFIG_FILE);

        let yaml_str = if config_file.exists() {
            read_from_file(config_file.as_path())
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
        } else if old_config_file.exists() {
            read_from_file(old_config_file.as_path())
                .ok()
                .and_then(|b| String::from_utf8(b).ok())
        } else {
            None
        };

        let Some(yaml_str) = yaml_str else {
            return false;
        };
        let Ok(raw) = serde_yaml::from_str::<RawCliConfig>(&yaml_str) else {
            return false;
        };

        let profile_name = profile.unwrap_or(DEFAULT_PROFILE);
        raw.profiles
            .as_ref()
            .and_then(|p| p.get(profile_name))
            .is_some_and(|p| p.is_field_encrypted(field_name))
    }

    pub fn load_profile(
        profile: Option<&str>,
        mode: ConfigSearchMode,
    ) -> CliTypedResult<Option<ProfileConfig>> {
        let mut config = Self::load(mode)?;

        // If no profile was given, use `default`
        if let Some(profile) = profile {
            if let Some(account_profile) = config.remove_profile(profile) {
                Ok(Some(account_profile))
            } else {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} not found",
                    profile
                )))
            }
        } else {
            Ok(config.remove_profile(DEFAULT_PROFILE))
        }
    }

    pub fn remove_profile(&mut self, profile: &str) -> Option<ProfileConfig> {
        if let Some(ref mut profiles) = self.profiles {
            profiles.remove(&profile.to_string())
        } else {
            None
        }
    }

    /// Two-phase save: convert typed profiles to raw strings, encrypt sensitive fields
    /// if encryption is enabled, then serialize to YAML.
    pub fn save(&self) -> CliTypedResult<()> {
        let aptos_folder = Self::aptos_folder(ConfigSearchMode::CurrentDir)?;

        // Create if it doesn't exist
        let no_dir = !aptos_folder.exists();
        create_dir_if_not_exist(aptos_folder.as_path())?;

        // If the `.aptos/` doesn't exist, we'll add a .gitignore in it to ignore the config file
        // so people don't save their credentials...
        if no_dir {
            write_to_user_only_file(
                aptos_folder.join(GIT_IGNORE).as_path(),
                GIT_IGNORE,
                APTOS_FOLDER_GIT_IGNORE.as_bytes(),
            )?;
        }

        // Derive key if encryption is enabled, and verify it matches the stored key check
        let derived_key = if let Some(ref enc_config) = self.encryption {
            let password = get_password(enc_config, &aptos_folder)?;
            let key = DerivedKey::derive(&password, enc_config)?;
            if !key.verify_key_check(enc_config) {
                return Err(CliError::WrongPassword);
            }
            Some(key)
        } else {
            None
        };

        // Convert typed profiles to raw (encrypting sensitive fields if needed)
        let raw_profiles = self
            .profiles
            .as_ref()
            .map(|profiles| {
                profiles
                    .iter()
                    .map(|(name, profile)| {
                        let raw = profile_config_to_raw(profile, derived_key.as_ref())?;
                        Ok((name.clone(), raw))
                    })
                    .collect::<CliTypedResult<BTreeMap<String, _>>>()
            })
            .transpose()?;

        let raw_config = RawCliConfig {
            encryption: self.encryption.clone(),
            profiles: raw_profiles,
        };

        // Save over previous config file
        let config_file = aptos_folder.join(CONFIG_FILE);
        let config_bytes = serde_yaml::to_string(&raw_config).map_err(|err| {
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
    pub fn aptos_folder(mode: ConfigSearchMode) -> CliTypedResult<PathBuf> {
        let global_config = GlobalConfig::load()?;
        global_config.get_config_location(mode)
    }
}

// ── GlobalConfig ──

const GLOBAL_CONFIG_FILE: &str = "global_config.yaml";

/// A global configuration for global settings related to a user
#[derive(Serialize, DeserializeTrait, Debug, Default)]
pub struct GlobalConfig {
    /// Whether to be using Global or Workspace mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ConfigType>,
    /// Prompt response type
    #[serde(default)]
    pub default_prompt_response: PromptResponseType,
}

impl GlobalConfig {
    /// Fill in defaults for display via the CLI
    pub fn display(mut self) -> CliTypedResult<Self> {
        if self.config_type.is_none() {
            self.config_type = Some(ConfigType::default());
        }

        Ok(self)
    }

    pub fn load() -> CliTypedResult<Self> {
        let path = global_folder()?.join(GLOBAL_CONFIG_FILE);
        if path.exists() {
            serde_yaml::from_str(&String::from_utf8(read_from_file(path.as_path())?)?)
                .map_err(|e| CliError::UnexpectedError(e.to_string()))
        } else {
            // If we don't have a config, let's load the default
            // Let's create the file if it doesn't exist
            let config = GlobalConfig::default();
            config.save()?;
            Ok(config)
        }
    }

    /// Get the config location based on the type
    pub fn get_config_location(&self, mode: ConfigSearchMode) -> CliTypedResult<PathBuf> {
        match self.config_type.unwrap_or_default() {
            ConfigType::Global => global_folder(),
            ConfigType::Workspace => find_workspace_config(current_dir()?, mode),
        }
    }

    /// Get the prompt options from global config
    pub fn get_default_prompt_response(&self) -> Option<bool> {
        match self.default_prompt_response {
            PromptResponseType::Prompt => None,    // prompt
            PromptResponseType::Yes => Some(true), // assume_yes
            PromptResponseType::No => Some(false), // assume_no
        }
    }

    pub fn save(&self) -> CliTypedResult<()> {
        let global_folder = global_folder()?;
        create_dir_if_not_exist(global_folder.as_path())?;

        let yaml =
            serde_yaml::to_string(&self).map_err(|e| CliError::UnexpectedError(e.to_string()))?;
        write_to_user_only_file(
            global_folder.join(GLOBAL_CONFIG_FILE).as_path(),
            "Global Config",
            yaml.as_bytes(),
        )?;
        // Let's also write a .gitignore that ignores this folder
        write_to_user_only_file(
            global_folder.join(GIT_IGNORE).as_path(),
            ".gitignore",
            APTOS_FOLDER_GIT_IGNORE.as_bytes(),
        )
    }
}

// ── Helper functions ──

pub fn global_folder() -> CliTypedResult<PathBuf> {
    if let Some(dir) = dirs::home_dir() {
        Ok(dir.join(CONFIG_FOLDER))
    } else {
        Err(CliError::UnexpectedError(
            "Unable to retrieve home directory".to_string(),
        ))
    }
}

pub fn find_workspace_config(
    starting_path: PathBuf,
    mode: ConfigSearchMode,
) -> CliTypedResult<PathBuf> {
    match mode {
        ConfigSearchMode::CurrentDir => Ok(starting_path.join(CONFIG_FOLDER)),
        ConfigSearchMode::CurrentDirAndParents => {
            let mut current_path = starting_path.clone();
            loop {
                current_path.push(CONFIG_FOLDER);
                if current_path.is_dir() {
                    break Ok(current_path);
                } else if !(current_path.pop() && current_path.pop()) {
                    // If we aren't able to find the folder, we'll create a new one right here
                    break Ok(starting_path.join(CONFIG_FOLDER));
                }
            }
        },
    }
}

// ── ConfigType ──

const GLOBAL: &str = "global";
const WORKSPACE: &str = "workspace";

/// A configuration for where to place and use the config
///
/// Workspace allows for multiple configs based on location, where
/// Global allows for one config for every part of the code
#[derive(Debug, Copy, Clone, Default, Serialize, DeserializeTrait, ValueEnum)]
pub enum ConfigType {
    /// Per system user configuration put in `<HOME>/.aptos`
    Global,
    /// Per directory configuration put in `<CURRENT_DIR>/.aptos`
    #[default]
    Workspace,
}

impl Display for ConfigType {
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

// ── PromptResponseType ──

const PROMPT: &str = "prompt";
const ASSUME_YES: &str = "yes";
const ASSUME_NO: &str = "no";

/// A configuration for how to expect the prompt response
///
/// Option can be one of ["yes", "no", "prompt"], "yes" runs cli with "--assume-yes", where
/// "no" runs cli with "--assume-no", default: "prompt"
#[derive(Debug, Copy, Clone, Default, Serialize, DeserializeTrait, ValueEnum)]
pub enum PromptResponseType {
    /// normal prompt
    #[default]
    Prompt,
    /// `--assume-yes`
    Yes,
    /// `--assume-no`
    No,
}

impl Display for PromptResponseType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            PromptResponseType::Prompt => PROMPT,
            PromptResponseType::Yes => ASSUME_YES,
            PromptResponseType::No => ASSUME_NO,
        })
    }
}

impl FromStr for PromptResponseType {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            PROMPT => Ok(Self::Prompt),
            ASSUME_YES => Ok(Self::Yes),
            ASSUME_NO => Ok(Self::No),
            _ => Err(CliError::CommandArgumentError(
                "Invalid prompt response type, must be one of [yes, no, prompt]".to_string(),
            )),
        }
    }
}
