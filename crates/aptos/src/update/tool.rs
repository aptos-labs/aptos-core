// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{check_if_update_required, helpers::InstallationMethod};
use crate::common::{
    types::{CliCommand, CliTypedResult},
    utils::cli_build_information,
};
use anyhow::{anyhow, Context};
use aptos_build_info::BUILD_OS;
use async_trait::async_trait;
use clap::Parser;
use self_update::{backends::github::Update, cargo_crate_version, Status};
use std::process::Command;

/// Update the CLI itself
///
/// This can be used to update the CLI to the latest version. This is useful if you
/// installed the CLI via the install script / by downloading the binary directly.
#[derive(Debug, Parser)]
pub struct UpdateTool {
    /// The owner of the repo to download the binary from.
    #[clap(long, default_value = "movementlabsxyz")]
    repo_owner: String,

    /// The name of the repo to download the binary from.
    #[clap(long, default_value = "aptos-core")]
    repo_name: String,
}

impl UpdateTool {
    // Out of the box this crate assumes that you have releases named a specific way
    // with the crate name, version, and target triple in a specific format. We don't
    // do this with our releases, we have other GitHub releases beyond just the CLI,
    // and we don't build for all major target triples, so we have to do some of the
    // work ourselves first to figure out what the latest version of the CLI is and
    // which binary to download based on the current OS. Then we can plug that into
    // the library which takes care of the rest.
    fn update(&self) -> CliTypedResult<String> {
        let installation_method =
            InstallationMethod::from_env().context("Failed to determine installation method")?;
        match installation_method {
            InstallationMethod::Source => {
                return Err(
                    anyhow!("Detected this CLI was built from source, refusing to update").into(),
                );
            },
            InstallationMethod::Homebrew => {
                return Err(anyhow!(
                    "Detected this CLI comes from homebrew, use `brew upgrade movement` instead"
                )
                .into());
            },
            InstallationMethod::Other => {},
        }

        let info = check_if_update_required(&self.repo_owner, &self.repo_name)?;
        if !info.update_required {
            return Ok(format!("CLI already up to date (v{})", info.latest_version));
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
            wildcard => return Err(anyhow!("Self-updating is not supported on your OS right now, please download the binary manually: {}", wildcard).into()),
        };

        // Build a new configuration that will direct the library to download the
        // binary with the target version tag and target that we determined above.
        let config = Update::configure()
            .repo_owner(&self.repo_owner)
            .repo_name(&self.repo_name)
            .bin_name("movement")
            .current_version(cargo_crate_version!())
            .target_version_tag(&info.latest_version_tag)
            .target(target)
            .build()
            .map_err(|e| anyhow!("Failed to build self-update configuration: {:#}", e))?;

        // Update the binary.
        let result = config
            .update()
            .map_err(|e| anyhow!("Failed to update Movement CLI: {:#}", e))?;

        let message = match result {
            Status::UpToDate(_) => panic!("We should have caught this already"),
            Status::Updated(_) => format!(
                "Successfully updated from v{} to v{}",
                info.current_version, info.latest_version
            ),
        };

        Ok(message)
    }
}

#[async_trait]
impl CliCommand<String> for UpdateTool {
    fn command_name(&self) -> &'static str {
        "Update"
    }

    async fn execute(self) -> CliTypedResult<String> {
        tokio::task::spawn_blocking(move || self.update())
            .await
            .context("Failed to self-update Movement CLI")?
    }
}
