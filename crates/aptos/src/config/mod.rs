// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{
            CliCommand, CliConfig, CliError, CliResult, CliTypedResult, ConfigSearchMode,
            ProfileSummary,
        },
        config::{ConfigType, GlobalConfig, PromptResponseType}
    },
    Tool,
};
use aptos_cli_common::generate_cli_completions;
use aptos_crypto::ValidCryptoMaterialStringExt;
use async_trait::async_trait;
use clap::Parser;
use clap_complete::Shell;
use std::{collections::BTreeMap, path::PathBuf};

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

        config.save()?;
        config.display()
    }
}

/// Show the private key for the given profile
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
                if let Some(private_key) = &profile.private_key {
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
