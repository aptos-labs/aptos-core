// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Context, Result};
use self_update::{backends::github::ReleaseList, cargo_crate_version, version::bump_is_greater};

#[derive(Debug)]
pub struct UpdateRequiredInfo {
    pub update_required: bool,
    pub current_version: String,
    pub latest_version: String,
    pub latest_version_tag: String,
}

/// Return information about whether an update is required.
pub fn check_if_update_required(repo_owner: &str, repo_name: &str) -> Result<UpdateRequiredInfo> {
    // Build a configuration for determining the latest release.
    let config = ReleaseList::configure()
        .repo_owner(repo_owner)
        .repo_name(repo_name)
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
    let latest_version_tag = latest_release.version;
    let latest_version = latest_version_tag.split("-v").last().unwrap();

    // Return early if we're up to date already.
    let current_version = cargo_crate_version!();
    let update_required = bump_is_greater(current_version, latest_version)
        .context("Failed to compare current and latest CLI versions")?;

    Ok(UpdateRequiredInfo {
        update_required,
        current_version: current_version.to_string(),
        latest_version: latest_version.to_string(),
        latest_version_tag,
    })
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
