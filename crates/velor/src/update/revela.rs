// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{update_binary, BinaryUpdater, UpdateRequiredInfo};
use crate::{
    common::types::{CliCommand, CliTypedResult, PromptOptions},
    update::update_helper::{build_updater, get_path},
};
use anyhow::{Context, Result};
use async_trait::async_trait;
use clap::Parser;
use self_update::update::ReleaseUpdate;
use std::path::PathBuf;

const REVELA_BINARY_NAME: &str = "revela";
const TARGET_REVELA_VERSION: &str = "1.0.0";

const REVELA_EXE_ENV: &str = "REVELA_EXE";
#[cfg(target_os = "windows")]
const REVELA_EXE: &str = "revela.exe";
#[cfg(not(target_os = "windows"))]
const REVELA_EXE: &str = "revela";

/// Update Revela, the tool used for decompilation.
#[derive(Debug, Parser)]
pub struct RevelaUpdateTool {
    /// The owner of the repo to download the binary from.
    #[clap(long, default_value = "verichains")]
    repo_owner: String,

    /// The name of the repo to download the binary from.
    #[clap(long, default_value = "revela")]
    repo_name: String,

    /// The version to install, e.g. 1.0.1. Use with caution, the default value is a
    /// version that is tested for compatibility with the version of the CLI you are
    /// using.
    #[clap(long, default_value = TARGET_REVELA_VERSION)]
    target_version: String,

    /// Where to install the binary. Make sure this directory is on your PATH. If not
    /// given we will put it in a standard location for your OS that the CLI will use
    /// later when the tool is required.
    #[clap(long)]
    install_dir: Option<PathBuf>,

    /// If set, it will check if there are updates for the tool, but not actually update
    #[clap(long, default_value_t = false)]
    check: bool,

    #[clap(flatten)]
    pub prompt_options: PromptOptions,
}

impl BinaryUpdater for RevelaUpdateTool {
    fn check(&self) -> bool {
        self.check
    }

    fn pretty_name(&self) -> String {
        "Revela".to_string()
    }

    /// Return information about whether an update is required.
    fn get_update_info(&self) -> Result<UpdateRequiredInfo> {
        // Get the current version, if any.
        let revela_path = get_revela_path();
        let current_version = match revela_path {
            Ok(path) => {
                let output = std::process::Command::new(path)
                    .arg("--version")
                    .output()
                    .context("Failed to get current version of Revela")?;
                let stdout = String::from_utf8(output.stdout)
                    .context("Failed to parse current version of Revela as UTF-8")?;
                let current_version = stdout
                    .split_whitespace()
                    .nth(1)
                    .map(|s| s.to_string())
                    .context("Failed to extract version number from command output")?;
                Some(current_version.trim_start_matches('v').to_string())
            },
            Err(_) => None,
        };

        // Strip v prefix from target version if present.
        let target_version = self.target_version.trim_start_matches('v').to_string();

        Ok(UpdateRequiredInfo {
            current_version,
            target_version,
        })
    }

    fn build_updater(&self, info: &UpdateRequiredInfo) -> Result<Box<dyn ReleaseUpdate>> {
        build_updater(
            info,
            self.install_dir.clone(),
            self.repo_owner.clone(),
            self.repo_name.clone(),
            REVELA_BINARY_NAME,
            "unknown-linux-gnu",
            "apple-darwin",
            "pc-windows-gnu",
            self.prompt_options.assume_yes,
        )
    }
}

#[async_trait]
impl CliCommand<String> for RevelaUpdateTool {
    fn command_name(&self) -> &'static str {
        "UpdateRevela"
    }

    async fn execute(self) -> CliTypedResult<String> {
        update_binary(self).await
    }
}

pub fn get_revela_path() -> Result<PathBuf> {
    get_path(
        "decompiler",
        REVELA_EXE_ENV,
        REVELA_BINARY_NAME,
        REVELA_EXE,
        false,
    )
}
