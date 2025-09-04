// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    cli_build_information,
    update::{get_additional_binaries_dir, UpdateRequiredInfo},
};
use anyhow::{anyhow, bail, Context, Result};
use velor_build_info::BUILD_OS;
use self_update::{backends::github::Update, update::ReleaseUpdate};
use std::path::PathBuf;

#[cfg(target_arch = "x86_64")]
pub fn get_arch() -> &'static str {
    "x86_64"
}

#[cfg(target_arch = "aarch64")]
pub fn get_arch() -> &'static str {
    "aarch64"
}

#[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
pub fn get_arch() -> &'static str {
    unimplemented!("Self-updating is not supported on your CPU architecture right now, please download the binary manually")
}

pub fn build_updater(
    info: &UpdateRequiredInfo,
    install_dir: Option<PathBuf>,
    repo_owner: String,
    repo_name: String,
    binary_name: &str,
    linux_name: &str,
    mac_os_name: &str,
    windows_name: &str,
    assume_yes: bool,
) -> Result<Box<dyn ReleaseUpdate>> {
    // Determine the target we should download based on how the CLI itself was built.
    let arch_str = get_arch();
    let build_info = cli_build_information();
    let target = match build_info.get(BUILD_OS).context("Failed to determine build info of current CLI")?.as_str() {
        "linux-aarch64" | "linux-x86_64" => linux_name,
        "macos-aarch64" | "macos-x86_64" => mac_os_name,
        "windows-x86_64" => windows_name,
        wildcard => bail!("Self-updating is not supported on your OS ({}) right now, please download the binary manually", wildcard),
    };

    let target = format!("{}-{}", arch_str, target);

    let install_dir = match install_dir.clone() {
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
        .bin_name(binary_name)
        .repo_owner(&repo_owner)
        .repo_name(&repo_name)
        .current_version(current_version)
        .target_version_tag(&format!("v{}", info.target_version))
        .target(&target)
        .no_confirm(assume_yes)
        .build()
        .map_err(|e| anyhow!("Failed to build self-update configuration: {:#}", e))
}

pub fn get_path(
    name: &str,
    exe_env: &str,
    binary_name: &str,
    exe: &str,
    find_in_path: bool,
) -> Result<PathBuf> {
    // Look at the environment variable first.
    if let Ok(path) = std::env::var(exe_env) {
        return Ok(PathBuf::from(path));
    }

    // See if it is present in the path where we usually install additional binaries.
    let path = get_additional_binaries_dir().join(binary_name);
    if path.exists() && path.is_file() {
        return Ok(path);
    }

    if find_in_path {
        // See if we can find the binary in the PATH.
        if let Some(path) = pathsearch::find_executable_in_path(exe) {
            return Ok(path);
        }
    }

    Err(anyhow!(
        "Cannot locate the {} executable. \
            Environment variable `{}` is not set, and `{}` is not in the PATH. \
            Try running `velor update {}` to download it and then \
            updating the environment variable `{}` or adding the executable to PATH",
        name,
        exe_env,
        exe,
        exe,
        exe_env
    ))
}
