// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{
        types::{
            CliCommand, CliConfig, CliError, CliResult, CliTypedResult, ConfigSearchMode,
            ProfileSummary, CONFIG_FOLDER,
        },
        utils::{create_dir_if_not_exist, current_dir, read_from_file, write_to_user_only_file},
    },
    genesis::git::{from_yaml, to_yaml},
    Tool,
};
use aptos_cli_common::generate_cli_completions;
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
    DeleteProfile(DeleteProfile),
    GenerateShellCompletions(GenerateShellCompletions),
    RenameProfile(RenameProfile),
    SetGlobalConfig(SetGlobalConfig),
    ShowGlobalConfig(ShowGlobalConfig),
    ShowPrivateKey(ShowPrivateKey),
    ShowProfiles(ShowProfiles),
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
                    Ok(format!("0x{}", hex::encode(private_key.to_bytes())))
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
#[derive(Debug, Copy, Clone, Serialize, Deserialize, ValueEnum)]
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

const PROMPT: &str = "prompt";
const ASSUME_YES: &str = "yes";
const ASSUME_NO: &str = "no";

/// A configuration for how to expect the prompt response
///
/// Option can be one of ["yes", "no", "prompt"], "yes" runs cli with "--assume-yes", where
/// "no" runs cli with "--assume-no", default: "prompt"
#[derive(Debug, Copy, Clone, Serialize, Deserialize, ValueEnum)]
pub enum PromptResponseType {
    /// normal prompt
    Prompt,
    /// `--assume-yes`
    Yes,
    /// `--assume-no`
    No,
}

impl Default for PromptResponseType {
    fn default() -> Self {
        Self::Prompt
    }
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
