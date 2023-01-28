// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Context, Result};
use self_update::{backends::github::Update, cargo_crate_version, version::bump_is_greater};

pub struct UpdateRequiredInfo {
    pub update_required: bool,
    pub current_version: String,
    pub latest_version: String,
    pub latest_version_tag: String,
}

/// Return information about whether an update is required.
pub fn check_if_update_required(repo_owner: &str, repo_name: &str) -> Result<UpdateRequiredInfo> {
    // Build a configuration for determining the latest release.
    let config = Update::configure()
        .repo_owner(repo_owner)
        .repo_name(repo_name)
        .bin_name("aptos")
        .current_version(cargo_crate_version!())
        .build()
        .map_err(|e| anyhow!("Failed to build self-update configuration: {:#}", e))?;

    // Get information about the latest release.
    let latest_release = config
        .get_latest_release()
        .map_err(|e| anyhow!("Failed to lookup latest release: {:#}", e))?;
    let latest_version_tag = latest_release.version;
    let latest_version = latest_version_tag.split("-v").last().unwrap();

    // Return early if we're up to date already.
    let current_version = config.current_version();
    let update_required = bump_is_greater(&current_version, latest_version)
        .context("Failed to compare current and latest CLI versions")?;

    Ok(UpdateRequiredInfo {
        update_required,
        current_version,
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
