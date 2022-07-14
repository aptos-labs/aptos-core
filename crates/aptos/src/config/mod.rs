// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::common::types::{CliCommand, CliError, CliResult, CliTypedResult};
use crate::Tool;
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
}

impl ConfigTool {
    pub async fn execute(self) -> CliResult {
        match self {
            ConfigTool::Init(tool) => tool.execute_serialized_success().await,
            ConfigTool::GenerateShellCompletions(tool) => tool.execute_serialized_success().await,
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
        "generate-shell-completions"
    }

    async fn execute(self) -> CliTypedResult<()> {
        let mut command = Tool::command();
        let mut file = std::fs::File::create(self.output_file.as_path())
            .map_err(|err| CliError::IO(self.output_file.display().to_string(), err))?;
        generate(self.shell, &mut command, "aptos".to_string(), &mut file);
        Ok(())
    }
}
