// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    common::{
        types::{
            CliCommand, CliConfig, CliError, CliResult, CliTypedResult, ConfigSearchMode,
            ProfileSummary, APTOS_FOLDER_GIT_IGNORE, CONFIG_FOLDER, GIT_IGNORE,
        },
        utils::{create_dir_if_not_exist, current_dir, read_from_file, write_to_user_only_file},
    },
    genesis::git::{from_yaml, to_yaml},
    Tool,
};
use aptos_cli_common::generate_cli_completions;
use aptos_crypto::ValidCryptoMaterialStringExt;
use async_trait::async_trait;
use clap::{Parser, ValueEnum};
use clap_complete::Shell;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt::Formatter, path::PathBuf, str::FromStr};

/// Tool for interacting with configuration of the Aptos CLI tool
///
/// This tool handles the global configuration of the CLI tool for
/// default configuration, and user specific settings.
#[derive(Parser)]
pub enum ConfigTool {
    GenerateShellCompletions(GenerateShellCompletions),
    ShowGlobalConfig(ShowGlobalConfig),
    SetGlobalConfig(SetGlobalConfig),
    ShowProfiles(ShowProfiles),
    ShowPrivateKey(ShowPrivateKey),
    RenameProfile(RenameProfile),
    DeleteProfile(DeleteProfile),
    EncryptCredentials(EncryptCredentials),
    DecryptCredentials(DecryptCredentials),
    RotatePassphrase(RotatePassphrase),
    StoreInKeychain(StoreInKeychain),
    RemoveFromKeychain(RemoveFromKeychain),
}

impl ConfigTool {
    pub async fn execute(self) -> CliResult {
        match self {
            ConfigTool::DeleteProfile(tool) => tool.execute_serialized().await,
            ConfigTool::GenerateShellCompletions(tool) => tool.execute_serialized_success().await,
            ConfigTool::RenameProfile(tool) => tool.execute_serialized().await,
            ConfigTool::SetGlobalConfig(tool) => tool.execute_serialized().await,
            ConfigTool::ShowGlobalConfig(tool) => tool.execute_serialized().await,
            ConfigTool::ShowPrivateKey(tool) => tool.execute_serialized().await,
            ConfigTool::ShowProfiles(tool) => tool.execute_serialized().await,
            ConfigTool::EncryptCredentials(tool) => tool.execute_serialized().await,
            ConfigTool::DecryptCredentials(tool) => tool.execute_serialized().await,
            ConfigTool::RotatePassphrase(tool) => tool.execute_serialized().await,
            ConfigTool::StoreInKeychain(tool) => tool.execute_serialized().await,
            ConfigTool::RemoveFromKeychain(tool) => tool.execute_serialized().await,
        }
    }
}

/// Generate shell completion files
///
/// First generate the completion file, then follow the shell specific directions on how
/// to install the completion file.
#[derive(Parser)]
pub struct GenerateShellCompletions {
    /// Shell to generate completions
    #[clap(long, value_enum, ignore_case = true)]
    shell: Shell,

    /// File to output shell completions to
    #[clap(long, value_parser)]
    output_file: PathBuf,
}

#[async_trait]
impl CliCommand<()> for GenerateShellCompletions {
    fn command_name(&self) -> &'static str {
        "GenerateShellCompletions"
    }

    async fn execute(self) -> CliTypedResult<()> {
        generate_cli_completions::<Tool>("aptos", self.shell, self.output_file.as_path())
            .map_err(|err| CliError::IO(self.output_file.display().to_string(), err))
    }
}

/// Set global configuration settings
///
/// Any configuration flags that are not provided will not be changed
#[derive(Parser, Debug)]
pub struct SetGlobalConfig {
    /// A configuration for where to place and use the config
    ///
    /// `Workspace` will put the `.aptos/` folder in the current directory, where
    /// `Global` will put the `.aptos/` folder in your home directory
    #[clap(long)]
    config_type: Option<ConfigType>,
    /// A configuration for how to expect the prompt response
    ///
    /// Option can be one of ["yes", "no", "prompt"], "yes" runs cli with "--assume-yes", where
    /// "no" runs cli with "--assume-no", default: "prompt"
    #[clap(long)]
    default_prompt_response: Option<PromptResponseType>,
    /// A configuration for credential encryption at rest
    ///
    /// Option can be one of ["disabled", "enabled", "keychain", "prompt"]
    /// - "disabled": Do not encrypt credentials (default)
    /// - "enabled": Always encrypt credentials when creating profiles
    /// - "keychain": Store credentials in system keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
    /// - "prompt": Ask whether to encrypt when creating profiles
    #[clap(long)]
    credential_encryption: Option<CredentialEncryptionType>,
}

#[async_trait]
impl CliCommand<GlobalConfig> for SetGlobalConfig {
    fn command_name(&self) -> &'static str {
        "SetGlobalConfig"
    }

    async fn execute(self) -> CliTypedResult<GlobalConfig> {
        // Load the global config
        let mut config = GlobalConfig::load()?;

        // Enable all features that are actually listed
        if let Some(config_type) = self.config_type {
            config.config_type = Some(config_type);
        }

        if let Some(default_prompt_response) = self.default_prompt_response {
            config.default_prompt_response = default_prompt_response;
        }

        if let Some(credential_encryption) = self.credential_encryption {
            config.credential_encryption = credential_encryption;
        }

        config.save()?;
        config.display()
    }
}

/// Show the private key for the given profile
///
/// If the private key is encrypted, you will be prompted for the passphrase
/// or it will use the APTOS_CLI_PASSPHRASE environment variable.
#[derive(Parser, Debug)]
pub struct ShowPrivateKey {
    /// Which profile's private key to show
    #[clap(long)]
    profile: String,
}

#[async_trait]
impl CliCommand<String> for ShowPrivateKey {
    fn command_name(&self) -> &'static str {
        "ShowPrivateKey"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let config = CliConfig::load(ConfigSearchMode::CurrentDir)?;

        if let Some(profiles) = &config.profiles {
            if let Some(profile) = profiles.get(&self.profile.clone()) {
                // Use get_private_key() which handles both encrypted and plaintext keys
                if let Some(private_key) = profile.get_private_key()? {
                    Ok(private_key.to_aip_80_string()?)
                } else {
                    Err(CliError::CommandArgumentError(format!(
                        "Profile {} does not have a private key",
                        self.profile
                    )))
                }
            } else {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} does not exist",
                    self.profile
                )))
            }
        } else {
            Err(CliError::CommandArgumentError(
                "Config has no profiles".to_string(),
            ))
        }
    }
}

/// Shows the current profiles available
///
/// This will only show public information and will not show
/// private information
#[derive(Parser, Debug)]
pub struct ShowProfiles {
    /// Which profile to show
    ///
    /// If provided, show only this profile
    #[clap(long)]
    profile: Option<String>,
}

#[async_trait]
impl CliCommand<BTreeMap<String, ProfileSummary>> for ShowProfiles {
    fn command_name(&self) -> &'static str {
        "ShowProfiles"
    }

    async fn execute(self) -> CliTypedResult<BTreeMap<String, ProfileSummary>> {
        // Load the profile config
        let config = CliConfig::load(ConfigSearchMode::CurrentDir)?;
        Ok(config
            .profiles
            .unwrap_or_default()
            .into_iter()
            .filter(|(key, _)| {
                if let Some(ref profile) = self.profile {
                    profile == key
                } else {
                    true
                }
            })
            .map(|(key, profile)| (key, ProfileSummary::from(&profile)))
            .collect())
    }
}

/// Delete the specified profile.
#[derive(Parser, Debug)]
pub struct DeleteProfile {
    /// Which profile to delete
    #[clap(long)]
    profile: String,
}

#[async_trait]
impl CliCommand<String> for DeleteProfile {
    fn command_name(&self) -> &'static str {
        "DeleteProfile"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let mut config = CliConfig::load(ConfigSearchMode::CurrentDir)?;

        if let Some(profiles) = &mut config.profiles {
            if profiles.remove(&self.profile).is_none() {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} does not exist",
                    self.profile
                )))
            } else {
                config.save().map_err(|err| {
                    CliError::UnexpectedError(format!(
                        "Unable to save config after deleting profile: {}",
                        err,
                    ))
                })?;
                Ok(format!("Deleted profile {}", self.profile))
            }
        } else {
            Err(CliError::CommandArgumentError(
                "Config has no profiles".to_string(),
            ))
        }
    }
}

/// Rename the specified profile.
#[derive(Parser, Debug)]
pub struct RenameProfile {
    /// Which profile to rename
    #[clap(long)]
    profile: String,

    /// New profile name
    #[clap(long)]
    new_profile_name: String,
}

#[async_trait]
impl CliCommand<String> for RenameProfile {
    fn command_name(&self) -> &'static str {
        "RenameProfile"
    }

    async fn execute(self) -> CliTypedResult<String> {
        let mut config = CliConfig::load(ConfigSearchMode::CurrentDir)?;

        if let Some(profiles) = &mut config.profiles {
            if profiles.contains_key(&self.new_profile_name.clone()) {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} already exists",
                    self.new_profile_name
                )))
            } else if let Some(profile_config) = profiles.remove(&self.profile) {
                profiles.insert(self.new_profile_name.clone(), profile_config);
                config.save().map_err(|err| {
                    CliError::UnexpectedError(format!(
                        "Unable to save config after renaming profile: {}",
                        err,
                    ))
                })?;
                Ok(format!(
                    "Renamed profile {} to {}",
                    self.profile, self.new_profile_name
                ))
            } else {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} does not exist",
                    self.profile
                )))
            }
        } else {
            Err(CliError::CommandArgumentError(
                "Config has no profiles".to_string(),
            ))
        }
    }
}

/// Shows the properties in the global config
#[derive(Parser, Debug)]
pub struct ShowGlobalConfig {}

#[async_trait]
impl CliCommand<GlobalConfig> for ShowGlobalConfig {
    fn command_name(&self) -> &'static str {
        "ShowGlobalConfig"
    }

    async fn execute(self) -> CliTypedResult<GlobalConfig> {
        // Load the global config
        let config = GlobalConfig::load()?;

        config.display()
    }
}

/// Encrypt the private key for a profile
///
/// This encrypts the private key stored in a profile using a passphrase.
/// Once encrypted, the passphrase will be required to use the private key.
/// You can set the APTOS_CLI_PASSPHRASE environment variable to avoid
/// being prompted for the passphrase on each operation.
#[derive(Parser, Debug)]
pub struct EncryptCredentials {
    /// Which profile's credentials to encrypt
    #[clap(long, default_value = "default")]
    profile: String,
}

#[async_trait]
impl CliCommand<String> for EncryptCredentials {
    fn command_name(&self) -> &'static str {
        "EncryptCredentials"
    }

    async fn execute(self) -> CliTypedResult<String> {
        use crate::common::utils::prompt_passphrase_with_confirmation;

        let mut config = CliConfig::load(ConfigSearchMode::CurrentDir)?;

        if let Some(profiles) = &mut config.profiles {
            if let Some(profile) = profiles.get_mut(&self.profile) {
                if profile.encrypted_private_key.is_some() {
                    return Err(CliError::CommandArgumentError(format!(
                        "Profile {} already has encrypted credentials",
                        self.profile
                    )));
                }

                if profile.private_key.is_none() {
                    return Err(CliError::CommandArgumentError(format!(
                        "Profile {} does not have a private key to encrypt",
                        self.profile
                    )));
                }

                let passphrase = prompt_passphrase_with_confirmation(
                    "Enter passphrase to encrypt credentials: ",
                )?;

                profile.encrypt_private_key(&passphrase)?;

                config.save().map_err(|err| {
                    CliError::UnexpectedError(format!(
                        "Unable to save config after encrypting credentials: {}",
                        err,
                    ))
                })?;

                Ok(format!(
                    "Successfully encrypted credentials for profile {}",
                    self.profile
                ))
            } else {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} does not exist",
                    self.profile
                )))
            }
        } else {
            Err(CliError::CommandArgumentError(
                "Config has no profiles".to_string(),
            ))
        }
    }
}

/// Decrypt the private key for a profile
///
/// This decrypts an encrypted private key and stores it in plaintext.
/// Note: Storing private keys in plaintext is less secure. Consider
/// keeping credentials encrypted and using the APTOS_CLI_PASSPHRASE
/// environment variable for automation use cases.
#[derive(Parser, Debug)]
pub struct DecryptCredentials {
    /// Which profile's credentials to decrypt
    #[clap(long, default_value = "default")]
    profile: String,
}

#[async_trait]
impl CliCommand<String> for DecryptCredentials {
    fn command_name(&self) -> &'static str {
        "DecryptCredentials"
    }

    async fn execute(self) -> CliTypedResult<String> {
        use crate::common::utils::read_passphrase;

        let mut config = CliConfig::load(ConfigSearchMode::CurrentDir)?;

        if let Some(profiles) = &mut config.profiles {
            if let Some(profile) = profiles.get_mut(&self.profile) {
                if profile.encrypted_private_key.is_none() {
                    return Err(CliError::CommandArgumentError(format!(
                        "Profile {} does not have encrypted credentials",
                        self.profile
                    )));
                }

                let passphrase = read_passphrase("Enter passphrase to decrypt credentials: ")?;

                profile.decrypt_private_key(&passphrase)?;

                config.save().map_err(|err| {
                    CliError::UnexpectedError(format!(
                        "Unable to save config after decrypting credentials: {}",
                        err,
                    ))
                })?;

                Ok(format!(
                    "Successfully decrypted credentials for profile {}",
                    self.profile
                ))
            } else {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} does not exist",
                    self.profile
                )))
            }
        } else {
            Err(CliError::CommandArgumentError(
                "Config has no profiles".to_string(),
            ))
        }
    }
}

/// Change the passphrase for encrypted credentials
///
/// This re-encrypts the private key with a new passphrase.
#[derive(Parser, Debug)]
pub struct RotatePassphrase {
    /// Which profile's passphrase to rotate
    #[clap(long, default_value = "default")]
    profile: String,
}

#[async_trait]
impl CliCommand<String> for RotatePassphrase {
    fn command_name(&self) -> &'static str {
        "RotatePassphrase"
    }

    async fn execute(self) -> CliTypedResult<String> {
        use crate::common::utils::{prompt_passphrase_with_confirmation, read_passphrase};

        let mut config = CliConfig::load(ConfigSearchMode::CurrentDir)?;

        if let Some(profiles) = &mut config.profiles {
            if let Some(profile) = profiles.get_mut(&self.profile) {
                if profile.encrypted_private_key.is_none() {
                    return Err(CliError::CommandArgumentError(format!(
                        "Profile {} does not have encrypted credentials",
                        self.profile
                    )));
                }

                // Get the current passphrase
                let current_passphrase = read_passphrase("Enter current passphrase: ")?;

                // Decrypt with the current passphrase
                profile.decrypt_private_key(&current_passphrase)?;

                // Get the new passphrase
                let new_passphrase = prompt_passphrase_with_confirmation("Enter new passphrase: ")?;

                // Re-encrypt with the new passphrase
                profile.encrypt_private_key(&new_passphrase)?;

                config.save().map_err(|err| {
                    CliError::UnexpectedError(format!(
                        "Unable to save config after rotating passphrase: {}",
                        err,
                    ))
                })?;

                Ok(format!(
                    "Successfully rotated passphrase for profile {}",
                    self.profile
                ))
            } else {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} does not exist",
                    self.profile
                )))
            }
        } else {
            Err(CliError::CommandArgumentError(
                "Config has no profiles".to_string(),
            ))
        }
    }
}

/// Store the private key in the system keychain
///
/// This moves the private key from file-based storage to the system keychain
/// (macOS Keychain, Windows Credential Manager, or Linux Secret Service).
/// The keychain provides better security as the credentials are protected
/// by the OS security mechanisms and can integrate with biometrics.
#[derive(Parser, Debug)]
pub struct StoreInKeychain {
    /// Which profile's credentials to store in keychain
    #[clap(long, default_value = "default")]
    profile: String,
}

#[async_trait]
impl CliCommand<String> for StoreInKeychain {
    fn command_name(&self) -> &'static str {
        "StoreInKeychain"
    }

    async fn execute(self) -> CliTypedResult<String> {
        use crate::common::keychain::is_keychain_available;

        // Check if keychain is available
        if !is_keychain_available() {
            return Err(CliError::UnexpectedError(
                "System keychain is not available on this platform. \
                 On Linux, ensure the Secret Service (e.g., gnome-keyring) is running."
                    .to_string(),
            ));
        }

        let mut config = CliConfig::load(ConfigSearchMode::CurrentDir)?;

        if let Some(profiles) = &mut config.profiles {
            if let Some(profile) = profiles.get_mut(&self.profile) {
                if profile.keychain_entry.is_some() {
                    return Err(CliError::CommandArgumentError(format!(
                        "Profile {} already has credentials stored in keychain",
                        self.profile
                    )));
                }

                if !profile.has_private_key() {
                    return Err(CliError::CommandArgumentError(format!(
                        "Profile {} does not have a private key to store",
                        self.profile
                    )));
                }

                profile.store_in_keychain(&self.profile)?;

                config.save().map_err(|err| {
                    CliError::UnexpectedError(format!(
                        "Unable to save config after storing credentials in keychain: {}",
                        err,
                    ))
                })?;

                Ok(format!(
                    "Successfully stored credentials for profile {} in system keychain",
                    self.profile
                ))
            } else {
                Err(CliError::CommandArgumentError(format!(
                    "Profile {} does not exist",
                    self.profile
                )))
            }
        } else {
            Err(CliError::CommandArgumentError(
                "Config has no profiles".to_string(),
            ))
        }
    }
}

/// Remove the private key from the system keychain
///
/// This retrieves the private key from the system keychain and optionally
/// stores it back in the config file. You can choose to encrypt it with
/// a passphrase or store it in plaintext.
#[derive(Parser, Debug)]
pub struct RemoveFromKeychain {
    /// Which profile's credentials to remove from keychain
    #[clap(long, default_value = "default")]
    profile: String,

    /// Store the key as encrypted (with passphrase) after removing from keychain
    #[clap(long)]
    encrypt: bool,

    /// Store the key as plaintext after removing from keychain (less secure)
    #[clap(long)]
    plaintext: bool,
}

#[async_trait]
impl CliCommand<String> for RemoveFromKeychain {
    fn command_name(&self) -> &'static str {
        "RemoveFromKeychain"
    }

    async fn execute(self) -> CliTypedResult<String> {
        use crate::common::utils::prompt_passphrase_with_confirmation;

        // Validate mutually exclusive options
        if self.encrypt && self.plaintext {
            return Err(CliError::CommandArgumentError(
                "Cannot specify both --encrypt and --plaintext".to_string(),
            ));
        }

        let mut config = CliConfig::load(ConfigSearchMode::CurrentDir)?;

        // Use a block to limit the scope of the mutable borrow
        let storage_method = {
            let profiles = config.profiles.as_mut().ok_or_else(|| {
                CliError::CommandArgumentError("Config has no profiles".to_string())
            })?;

            let profile = profiles.get_mut(&self.profile).ok_or_else(|| {
                CliError::CommandArgumentError(format!("Profile {} does not exist", self.profile))
            })?;

            if profile.keychain_entry.is_none() {
                return Err(CliError::CommandArgumentError(format!(
                    "Profile {} does not have credentials stored in keychain",
                    self.profile
                )));
            }

            // Remove from keychain and keep plaintext temporarily
            profile.remove_from_keychain(true)?;

            // Now handle encryption if requested
            if self.encrypt {
                let passphrase = prompt_passphrase_with_confirmation(
                    "Enter passphrase to encrypt credentials: ",
                )?;
                profile.encrypt_private_key(&passphrase)?;
            } else if !self.plaintext {
                // Default behavior: ask user what to do
                eprintln!("The private key has been removed from the keychain.");
                eprintln!("Would you like to encrypt it with a passphrase? (recommended)");
                if crate::common::utils::prompt_yes("Encrypt credentials?") {
                    let passphrase = prompt_passphrase_with_confirmation(
                        "Enter passphrase to encrypt credentials: ",
                    )?;
                    profile.encrypt_private_key(&passphrase)?;
                } else {
                    eprintln!("Warning: Storing private key in plaintext is less secure.");
                }
            }

            // Determine storage method - this value will be returned from the block
            if profile.encrypted_private_key.is_some() {
                "encrypted"
            } else {
                "plaintext"
            }
        }; // Mutable borrow ends here

        config.save().map_err(|err| {
            CliError::UnexpectedError(format!(
                "Unable to save config after removing credentials from keychain: {}",
                err,
            ))
        })?;

        Ok(format!(
            "Successfully removed credentials for profile {} from keychain and stored as {}",
            self.profile, storage_method
        ))
    }
}

const GLOBAL_CONFIG_FILE: &str = "global_config.yaml";

/// A global configuration for global settings related to a user
#[derive(Serialize, Deserialize, Debug, Default)]
pub struct GlobalConfig {
    /// Whether to be using Global or Workspace mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_type: Option<ConfigType>,
    /// Prompt response type
    #[serde(default)]
    pub default_prompt_response: PromptResponseType,
    /// Whether to encrypt credentials by default when creating new profiles
    #[serde(default)]
    pub credential_encryption: CredentialEncryptionType,
}

/// Configuration for credential encryption behavior
#[derive(Debug, Copy, Clone, Serialize, Deserialize, ValueEnum, Default, PartialEq, Eq)]
pub enum CredentialEncryptionType {
    /// Do not encrypt credentials (default for backward compatibility)
    #[default]
    Disabled,
    /// Always encrypt credentials when creating or updating profiles
    Enabled,
    /// Store credentials in the system keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service)
    Keychain,
    /// Prompt the user to choose whether to encrypt credentials
    Prompt,
}

impl std::fmt::Display for CredentialEncryptionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            CredentialEncryptionType::Disabled => "disabled",
            CredentialEncryptionType::Enabled => "enabled",
            CredentialEncryptionType::Keychain => "keychain",
            CredentialEncryptionType::Prompt => "prompt",
        })
    }
}

impl FromStr for CredentialEncryptionType {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().trim() {
            "disabled" | "off" | "no" | "false" => Ok(Self::Disabled),
            "enabled" | "on" | "yes" | "true" => Ok(Self::Enabled),
            "keychain" | "keyring" | "system" => Ok(Self::Keychain),
            "prompt" | "ask" => Ok(Self::Prompt),
            _ => Err(CliError::CommandArgumentError(
                "Invalid credential encryption type, must be one of [disabled, enabled, keychain, prompt]"
                    .to_string(),
            )),
        }
    }
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
            from_yaml(&String::from_utf8(read_from_file(path.as_path())?)?)
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

    fn save(&self) -> CliTypedResult<()> {
        let global_folder = global_folder()?;
        create_dir_if_not_exist(global_folder.as_path())?;

        write_to_user_only_file(
            global_folder.join(GLOBAL_CONFIG_FILE).as_path(),
            "Global Config",
            &to_yaml(&self)?.into_bytes(),
        )?;
        // Let's also write a .gitignore that ignores this folder
        write_to_user_only_file(
            global_folder.join(GIT_IGNORE).as_path(),
            ".gitignore",
            APTOS_FOLDER_GIT_IGNORE.as_bytes(),
        )
    }
}

fn global_folder() -> CliTypedResult<PathBuf> {
    if let Some(dir) = dirs::home_dir() {
        Ok(dir.join(CONFIG_FOLDER))
    } else {
        Err(CliError::UnexpectedError(
            "Unable to retrieve home directory".to_string(),
        ))
    }
}

fn find_workspace_config(
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

const GLOBAL: &str = "global";
const WORKSPACE: &str = "workspace";

/// A configuration for where to place and use the config
///
/// Workspace allows for multiple configs based on location, where
/// Global allows for one config for every part of the code
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, ValueEnum)]
pub enum ConfigType {
    /// Per system user configuration put in `<HOME>/.aptos`
    Global,
    /// Per directory configuration put in `<CURRENT_DIR>/.aptos`
    // TODO: When we version up, we can change this to global
    #[default]
    Workspace,
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

const PROMPT: &str = "prompt";
const ASSUME_YES: &str = "yes";
const ASSUME_NO: &str = "no";

/// A configuration for how to expect the prompt response
///
/// Option can be one of ["yes", "no", "prompt"], "yes" runs cli with "--assume-yes", where
/// "no" runs cli with "--assume-no", default: "prompt"
#[derive(Debug, Copy, Clone, Default, Serialize, Deserialize, ValueEnum)]
pub enum PromptResponseType {
    /// normal prompt
    #[default]
    Prompt,
    /// `--assume-yes`
    Yes,
    /// `--assume-no`
    No,
}

impl std::fmt::Display for PromptResponseType {
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
