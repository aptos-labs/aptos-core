// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::CliCommand;
use crate::Tool;
use aptos_cli_base::config::{ConfigType, GlobalConfig};
use aptos_cli_base::types::{CliError, CliResult, CliTypedResult};
use async_trait::async_trait;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::{generate, Shell};
use std::path::PathBuf;

/// Tool for configuration of the CLI tool
///
#[derive(Parser)]
pub enum ConfigTool {
    Init(crate::common::init::InitTool),
    GenerateShellCompletions(GenerateShellCompletions),
    SetGlobalConfig(SetGlobalConfig),
    ShowGlobalConfig(ShowGlobalConfig),
}

impl ConfigTool {
    pub async fn execute(self) -> CliResult {
        match self {
            ConfigTool::Init(tool) => tool.execute_serialized_success().await,
            ConfigTool::GenerateShellCompletions(tool) => tool.execute_serialized_success().await,
            ConfigTool::SetGlobalConfig(tool) => tool.execute_serialized_success().await,
            ConfigTool::ShowGlobalConfig(tool) => tool.execute_serialized().await,
        }
    }
}

/// Generates shell completion files
///
/// First generate the completion file, then follow the shell specific directions on how
/// to install the completion file.
#[derive(Parser)]
pub struct GenerateShellCompletions {
    /// Shell to generate completions for one of [bash, elvish, powershell, zsh]
    #[clap(long)]
    shell: Shell,
    /// File to output shell completions to
    #[clap(long, parse(from_os_str))]
    output_file: PathBuf,
}

#[async_trait]
impl CliCommand<()> for GenerateShellCompletions {
    fn command_name(&self) -> &'static str {
        "GenerateShellCompletions"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let mut command = Tool::command();
        let mut file = std::fs::File::create(self.output_file.as_path())
            .map_err(|err| CliError::IO(self.output_file.display().to_string(), err))?;
        generate(self.shell, &mut command, "aptos".to_string(), &mut file);
        Ok(())
    }
}

/// Set global configuration settings
///
/// Any configuration flags that are not provided will not be changed
#[derive(Parser, Debug)]
pub struct SetGlobalConfig {
    /// A configuration for where to place and use the config
    ///
    /// Workspace allows for multiple configs based on location, where
    /// Global allows for one config for every part of the code
    #[clap(long)]
    config_type: Option<ConfigType>,
}

#[async_trait]
impl CliCommand<()> for SetGlobalConfig {
    fn command_name(&self) -> &'static str {
        "SetGlobalConfig"
    }

    async fn execute(self) -> CliTypedResult<()> {
        // Load the global config
        let mut config = GlobalConfig::load()?;

        // Enable all features that are actually listed
        if let Some(config_type) = self.config_type {
            config.config_type = config_type;
        }

        config.save()
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
        GlobalConfig::load()
    }
}
