// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// Out of the box the self_update crate assumes that you have releases named a
// specific way with the crate name, version, and target triple in a specific
// format. We don't do this with our releases, we have other GitHub releases beyond
// just the CLI, and we don't build for all major target triples, so we have to do
// some of the work ourselves first to figure out what the latest version of the
// CLI is and which binary to download based on the current OS. Then we can plug
// that into the library which takes care of the rest.

use super::{update_binary, BinaryUpdater, UpdateRequiredInfo};
use crate::common::{
    types::{CliCommand, CliTypedResult},
    utils::cli_build_information,
};
use anyhow::{anyhow, Context, Result};
use aptos_build_info::BUILD_OS;
use async_trait::async_trait;
use clap::Parser;
use self_update::{
    backends::github::{ReleaseList, Update},
    cargo_crate_version,
    update::ReleaseUpdate,
};
use std::process::Command;

/// Update the CLI itself
///
/// This can be used to update the CLI to the latest version. This is useful if you
/// installed the CLI via the install script / by downloading the binary directly.
#[derive(Debug, Parser)]
pub struct AptosUpdateTool {
    /// The owner of the repo to download the binary from.
    #[clap(long, default_value = "aptos-labs")]
    repo_owner: String,

    /// The name of the repo to download the binary from.
    #[clap(long, default_value = "aptos-core")]
    repo_name: String,

    /// If set, it will check if there are updates for the tool, but not actually update
    #[clap(long, default_value_t = false)]
    check: bool,
}

impl BinaryUpdater for AptosUpdateTool {
    fn check(&self) -> bool {
        self.check
    }

    fn pretty_name(&self) -> &'static str {
        "Aptos CLI"
    }

    /// Return information about whether an update is required.
    fn get_update_info(&self) -> Result<UpdateRequiredInfo> {
        // Build a configuration for determining the latest release.
        let config = ReleaseList::configure()
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .build()
            .map_err(|e| anyhow!("Failed to build configuration to fetch releases: {:#}", e))?;

        // Get the most recent releases.
        let releases = config
            .fetch()
            .map_err(|e| anyhow!("Failed to fetch releases: {:#}", e))?;

        // Find the latest release of the CLI, in which we filter for the CLI tag.
        // If the release isn't in the last 30 items (the default API page size)
        // this will fail. See https://github.com/aptos-labs/aptos-core/issues/6411.
        let mut releases = releases.into_iter();
        let latest_release = loop {
            let release = match releases.next() {
                Some(release) => release,
                None => return Err(anyhow!("Failed to find latest CLI release")),
            };
            if release.version.starts_with("aptos-cli-") {
                break release;
            }
        };
        let target_version = latest_release.version.split("-v").last().unwrap();

        // Return early if we're up to date already.
        let current_version = cargo_crate_version!();

        Ok(UpdateRequiredInfo {
            current_version: Some(current_version.to_string()),
            target_version: target_version.to_string(),
        })
    }

    fn build_updater(&self, info: &UpdateRequiredInfo) -> Result<Box<dyn ReleaseUpdate>> {
        let installation_method =
            InstallationMethod::from_env().context("Failed to determine installation method")?;
        match installation_method {
            InstallationMethod::Source => {
                return Err(anyhow!(
                    "Detected this CLI was built from source, refusing to update"
                ));
            },
            InstallationMethod::Homebrew => {
                return Err(anyhow!(
                    "Detected this CLI comes from homebrew, use `brew upgrade aptos` instead"
                ));
            },
            InstallationMethod::Other => {},
        }

        // Determine the target we should download. This is necessary because we don't
        // name our binary releases using the target triples nor do we build specifically
        // for all major triples, so we have to generalize to one of the binaries we do
        // happen to build. We figure this out based on what system the CLI was built on.
        let build_info = cli_build_information();
        let target = match build_info.get(BUILD_OS).context("Failed to determine build info of current CLI")?.as_str() {
            "linux-x86_64" => {
                // In the case of Linux, which build to use depends on the OpenSSL
                // library on the host machine. So we try to determine that here.
                // This code below parses the output of the `openssl version` command,
                // where the version string is the 1th (0-indexing) item in the string
                // when split by whitespace.
                let output = Command::new("openssl")
                .args(["version"])
                .output();
                let version = match output {
                    Ok(output) => {
                        let stdout = String::from_utf8(output.stdout).unwrap();
                        stdout.split_whitespace().collect::<Vec<&str>>()[1].to_string()
                    },
                    Err(e) => {
                        println!("Failed to determine OpenSSL version, assuming an older version: {:#}", e);
                        "1.0.0".to_string()
                    }
                };
                // On Ubuntu < 22.04 the bundled OpenSSL is version 1.x.x, whereas on
                // 22.04+ it is 3.x.x. Unfortunately if you build the CLI on a system
                // with one major version of OpenSSL, you cannot use it on a system
                // with a different version. Accordingly, if the current system uses
                // OpenSSL 3.x.x, we use the version of the CLI built on a system with
                // OpenSSL 3.x.x, meaning Ubuntu 22.04. Otherwise we use the one built
                // on 20.04.
                if version.starts_with('3') {
                    "Ubuntu-22.04-x86_64"
                } else {
                    "Ubuntu-x86_64"
                }
            },
            "macos-x86_64" => "MacOSX-x86_64",
            "windows-x86_64" => "Windows-x86_64",
            wildcard => return Err(anyhow!("Self-updating is not supported on your OS ({}) right now, please download the binary manually", wildcard)),
        };

        let current_version = match &info.current_version {
            Some(version) => version,
            None => unreachable!("current_version should always be Some at this point"),
        };

        // Build a new configuration that will direct the library to download the
        // binary with the target version tag and target that we determined above.
        Update::configure()
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .bin_name("aptos")
            .current_version(current_version)
            .target_version_tag(&format!("aptos-cli-v{}", info.target_version))
            .target(target)
            .build()
            .map_err(|e| anyhow!("Failed to build self-update configuration: {:#}", e))
    }
}

pub enum InstallationMethod {
    Source,
    Homebrew,
    Other,
}

impl InstallationMethod {
    pub fn from_env() -> Result<Self> {
        // Determine update instructions based on what we detect about the installation.
        let exe_path = std::env::current_exe()?;
        let installation_method = if exe_path.to_string_lossy().contains("brew") {
            InstallationMethod::Homebrew
        } else if exe_path.to_string_lossy().contains("target") {
            InstallationMethod::Source
        } else {
            InstallationMethod::Other
        };
        Ok(installation_method)
    }
}

#[async_trait]
impl CliCommand<String> for AptosUpdateTool {
    fn command_name(&self) -> &'static str {
        "UpdateAptos"
    }

    async fn execute(self) -> CliTypedResult<String> {
        update_binary(self).await
    }
}
