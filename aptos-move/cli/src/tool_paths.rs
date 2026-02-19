// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tool path resolution for external binaries (revela, movefmt).
//!
//! In the full Aptos CLI, these are managed by the update system.
//! In standalone mode, we search standard locations and PATH.

use anyhow::{anyhow, Result};
use std::path::PathBuf;

const REVELA_BINARY_NAME: &str = "revela";
const MOVEFMT_BINARY_NAME: &str = "movefmt";
const REVELA_EXE_ENV: &str = "REVELA_EXE";
const MOVEFMT_EXE_ENV: &str = "MOVEFMT_EXE";

fn get_path(name: &str, exe_env: &str, binary_name: &str) -> Result<PathBuf> {
    // Check environment variable first
    if let Ok(path) = std::env::var(exe_env) {
        return Ok(PathBuf::from(path));
    }

    // Check PATH
    if let Ok(output) = std::process::Command::new("which")
        .arg(binary_name)
        .output()
    {
        if output.status.success() {
            let path_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path_str.is_empty() {
                return Ok(PathBuf::from(path_str));
            }
        }
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
