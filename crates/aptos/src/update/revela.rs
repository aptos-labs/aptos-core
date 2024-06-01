// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{get_additional_binaries_dir, update_binary, BinaryUpdater, UpdateRequiredInfo};
use crate::common::{
    types::{CliCommand, CliTypedResult},
    utils::cli_build_information,
};
use anyhow::{anyhow, bail, Context, Result};
use aptos_build_info::BUILD_OS;
use async_trait::async_trait;
use clap::Parser;
use self_update::{backends::github::Update, update::ReleaseUpdate};
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
}

impl BinaryUpdater for RevelaUpdateTool {
    fn check(&self) -> bool {
        self.check
    }

    fn pretty_name(&self) -> &'static str {
        "Revela"
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
        // Determine the target we should download based on how the CLI itself was built.
        let arch_str = get_arch();
        let build_info = cli_build_information();
        let target = match build_info.get(BUILD_OS).context("Failed to determine build info of current CLI")?.as_str() {
            "linux-aarch64" | "linux-x86_64" => "unknown-linux-gnu",
            "macos-aarch64" | "macos-x86_64" => "apple-darwin",
            "windows-x86_64" => "pc-windows-gnu",
            wildcard => bail!("Self-updating is not supported on your OS ({}) right now, please download the binary manually", wildcard),
        };

        let target = format!("{}-{}", arch_str, target);

        let install_dir = match self.install_dir.clone() {
            Some(dir) => dir,
            None => {
                let dir = get_additional_binaries_dir();
                // Make the directory if it doesn't already exist.
                std::fs::create_dir_all(&dir)
                    .with_context(|| format!("Failed to create directory: {:?}", dir))?;
                dir
            },
        };

        let current_version = match &info.current_version {
            Some(version) => version,
            None => "0.0.0",
        };

        Update::configure()
            .bin_install_dir(install_dir)
            .bin_name(REVELA_BINARY_NAME)
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .current_version(current_version)
            .target_version_tag(&format!("v{}", info.target_version))
            .target(&target)
            .build()
            .map_err(|e| anyhow!("Failed to build self-update configuration: {:#}", e))
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

#[cfg(target_arch = "x86_64")]
fn get_arch() -> &'static str {
    "x86_64"
}

#[cfg(target_arch = "aarch64")]
fn get_arch() -> &'static str {
    "aarch64"
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
fn get_arch() -> &'static str {
    unimplemented!("Self-updating is not supported on your CPU architecture right now, please download the binary manually")
}

pub fn get_revela_path() -> Result<PathBuf> {
    // Look at the environment variable first.
    if let Ok(path) = std::env::var(REVELA_EXE_ENV) {
        return Ok(PathBuf::from(path));
    }

    // See if it is present in the path where we usually install additional binaries.
    let path = get_additional_binaries_dir().join(REVELA_BINARY_NAME);
    if path.exists() && path.is_file() {
        return Ok(path);
    }

    // See if we can find the binary in the PATH.
    if let Some(path) = pathsearch::find_executable_in_path(REVELA_EXE) {
        return Ok(path);
    }

    Err(anyhow!(
        "Cannot locate the decompiler executable. \
            Environment variable `{}` is not set, and `{}` is not in the PATH. \
            Try running `aptos update revela` to download it.",
        REVELA_EXE_ENV,
        REVELA_EXE
    ))
}
