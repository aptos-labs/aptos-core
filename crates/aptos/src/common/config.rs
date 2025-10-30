use std::fmt::Formatter;
use std::path::PathBuf;
use std::str::FromStr;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use crate::common::types::{CliError, CliTypedResult, ConfigSearchMode, APTOS_FOLDER_GIT_IGNORE, CONFIG_FOLDER, GIT_IGNORE};
use crate::common::utils::{create_dir_if_not_exist, current_dir, read_from_file, write_to_user_only_file};
use crate::common::yaml::{from_yaml, to_yaml};

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

    pub fn save(&self) -> CliTypedResult<()> {
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
