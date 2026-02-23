// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tool path resolution for external binaries (revela, movefmt).
//!
//! In the full Aptos CLI, these are managed by the update system which installs
//! them into `~/.local/bin` (Linux/macOS) or `~/.aptoscli/bin` (Windows).
//! This module checks the environment variable, the aptos-managed install
//! directory, and finally PATH.

use anyhow::{anyhow, Result};
use std::path::PathBuf;

const REVELA_BINARY_NAME: &str = "revela";
const MOVEFMT_BINARY_NAME: &str = "movefmt";
const REVELA_EXE_ENV: &str = "REVELA_EXE";
const MOVEFMT_EXE_ENV: &str = "MOVEFMT_EXE";

/// Returns the directory where `aptos update` installs additional binaries.
fn get_additional_binaries_dir() -> PathBuf {
    #[cfg(windows)]
    {
        let home_dir = std::env::var("USERPROFILE").unwrap_or_default();
        PathBuf::from(home_dir).join(".aptoscli/bin")
    }

    #[cfg(not(windows))]
    {
        let home_dir = std::env::var("HOME").unwrap_or_default();
        PathBuf::from(home_dir).join(".local/bin")
    }
}

fn get_path(name: &str, exe_env: &str, binary_name: &str) -> Result<PathBuf> {
    // Check environment variable first.
    if let Ok(path) = std::env::var(exe_env) {
        return Ok(PathBuf::from(path));
    }

    // Check the aptos-managed install directory (~/.local/bin or ~/.aptoscli/bin).
    let path = get_additional_binaries_dir().join(binary_name);
    if path.exists() && path.is_file() {
        return Ok(path);
    }

    // Search PATH using a portable lookup (works on both Unix and Windows).
    if let Some(path) = pathsearch::find_executable_in_path(binary_name) {
        return Ok(path);
    }

    Err(anyhow!(
        "Could not find `{}`. Please install it or set the {} environment variable. \
         With the full Aptos CLI, you can run `aptos update {}` to install it.",
        name,
        exe_env,
        binary_name,
    ))
}

pub fn get_revela_path() -> Result<PathBuf> {
    get_path(REVELA_BINARY_NAME, REVELA_EXE_ENV, REVELA_BINARY_NAME)
}

pub fn get_movefmt_path() -> Result<PathBuf> {
    get_path(MOVEFMT_BINARY_NAME, MOVEFMT_EXE_ENV, MOVEFMT_BINARY_NAME)
}
