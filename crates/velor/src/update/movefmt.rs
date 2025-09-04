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

const FORMATTER_BINARY_NAME: &str = "movefmt";
const TARGET_FORMATTER_VERSION: &str = "1.0.8";

const FORMATTER_EXE_ENV: &str = "FORMATTER_EXE";
#[cfg(target_os = "windows")]
const FORMATTER_EXE: &str = "movefmt.exe";
#[cfg(not(target_os = "windows"))]
const FORMATTER_EXE: &str = "movefmt";

/// Update Movefmt, the tool used for formatting Move code.
#[derive(Debug, Parser)]
pub struct FormatterUpdateTool {
    /// The owner of the repo to download the binary from.
    #[clap(long, default_value = "movebit")]
    repo_owner: String,

    /// The name of the repo to download the binary from.
    #[clap(long, default_value = "movefmt")]
    repo_name: String,

    /// The version to install, e.g. 1.0.1. Use with caution, the default value is a
    /// version that is tested for compatibility with the version of the CLI you are
    /// using.
    #[clap(long, default_value = TARGET_FORMATTER_VERSION)]
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

fn extract_movefmt_version(input: &str) -> String {
    use regex::Regex;
    let re = Regex::new(r"movefmt v\d+\.\d+\.\d+").unwrap();
    if let Some(caps) = re.captures(input) {
        let version = caps.get(0).unwrap().as_str().to_string();
        return version.trim_start_matches("movefmt v").to_string();
    }
    String::new()
}

impl BinaryUpdater for FormatterUpdateTool {
    fn check(&self) -> bool {
        self.check
    }

    fn pretty_name(&self) -> String {
        "movefmt".to_string()
    }

    /// Return information about whether an update is required.
    fn get_update_info(&self) -> Result<UpdateRequiredInfo> {
        // Get the current version, if any.
        let fmt_path = get_movefmt_path();
        let current_version = match fmt_path {
            Ok(path) => {
                let output = std::process::Command::new(path)
                    .arg("--version")
                    .output()
                    .context("Failed to get current version of movefmt")?;
                let stdout = String::from_utf8(output.stdout)
                    .context("Failed to parse current version of movefmt as UTF-8")?;
                let version = extract_movefmt_version(&stdout);
                if !version.is_empty() {
                    Some(version)
                } else {
                    None
                }
            },
            Err(_) => None,
        };

        Ok(UpdateRequiredInfo {
            current_version,
            target_version: self.target_version.trim_start_matches('v').to_string(),
        })
    }

    fn build_updater(&self, info: &UpdateRequiredInfo) -> Result<Box<dyn ReleaseUpdate>> {
        build_updater(
            info,
            self.install_dir.clone(),
            self.repo_owner.clone(),
            self.repo_name.clone(),
            FORMATTER_BINARY_NAME,
            "unknown-linux-gnu",
            "apple-darwin",
            "windows",
            self.prompt_options.assume_yes,
        )
    }
}

#[async_trait]
impl CliCommand<String> for FormatterUpdateTool {
    fn command_name(&self) -> &'static str {
        "UpdateMovefmt"
    }

    async fn execute(self) -> CliTypedResult<String> {
        update_binary(self).await
    }
}

pub fn get_movefmt_path() -> Result<PathBuf> {
    get_path(
        FORMATTER_BINARY_NAME,
        FORMATTER_EXE_ENV,
        FORMATTER_BINARY_NAME,
        FORMATTER_EXE,
        true,
    )
}
