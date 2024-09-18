// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{BinaryUpdater, UpdateRequiredInfo};
use crate::update::{
    prover_dependencies::{REPO_NAME, REPO_OWNER},
    update_helper::{build_updater, get_path},
};
use anyhow::{Context, Result};
use self_update::update::ReleaseUpdate;
use std::path::PathBuf;

/// Update Prover dependency.
#[derive(Debug)]
pub struct DependencyInstaller {
    pub binary_name: String,

    pub exe_name: String,

    pub env_var: String,

    pub version_match_string: String,

    pub version_option_string: String,

    pub target_version: String,

    pub install_dir: Option<PathBuf>,

    pub check: bool,
}

impl DependencyInstaller {
    fn extract_version(&self, input: &str) -> String {
        use regex::Regex;
        let version_format = format!(r"{}{}", self.version_match_string, r"\d+\.\d+\.\d+");
        let re = Regex::new(&version_format).unwrap();
        if let Some(caps) = re.captures(input) {
            let version = caps.get(0).unwrap().as_str().to_string();
            return version
                .trim_start_matches(&self.version_match_string)
                .to_string();
        }
        String::new()
    }

    pub fn get_path(&self) -> Result<PathBuf> {
        get_path(
            &self.binary_name,
            &self.env_var,
            &self.binary_name,
            &self.exe_name,
            false,
        )
    }
}

impl BinaryUpdater for DependencyInstaller {
    fn check(&self) -> bool {
        false
    }

    fn pretty_name(&self) -> String {
        self.binary_name.clone()
    }

    /// Return information about whether an update is required.
    fn get_update_info(&self) -> Result<UpdateRequiredInfo> {
        // Get the current version, if any.
        let dependency_path = self.get_path();
        let current_version = match dependency_path {
            Ok(path) if path.exists() => {
                let output = std::process::Command::new(path)
                    .arg(format!("{}version", self.version_option_string))
                    .output()
                    .context("Failed to get current version")?;
                let stdout = String::from_utf8(output.stdout)
                    .context("Failed to parse current version as UTF-8")?;
                let version = self.extract_version(&stdout);
                if !version.is_empty() {
                    Some(version)
                } else {
                    None
                }
            },
            _ => None,
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
            REPO_OWNER.to_string(),
            REPO_NAME.to_string(),
            &self.binary_name,
            "unknown-linux-gnu",
            "apple-darwin",
            "windows",
        )
    }
}
