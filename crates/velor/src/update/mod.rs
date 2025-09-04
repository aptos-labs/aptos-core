// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

// Note: We make use of the self_update crate, but as you can see in the case of
// Revela, this can also be used to install / update other binaries.

mod velor;
mod helpers;
mod move_mutation_test;
mod movefmt;
mod prover_dependencies;
mod prover_dependency_installer;
mod revela;
mod tool;
mod update_helper;

use crate::common::types::CliTypedResult;
use anyhow::{anyhow, Context, Result};
pub use helpers::get_additional_binaries_dir;
pub use movefmt::get_movefmt_path;
pub use revela::get_revela_path;
use self_update::{update::ReleaseUpdate, version::bump_is_greater, Status};
pub use tool::UpdateTool;

/// Things that implement this trait are able to update a binary.
trait BinaryUpdater {
    /// For checking the version but not updating
    fn check(&self) -> bool;

    /// Only used for messages we print to the user.
    fn pretty_name(&self) -> String;

    /// Return information about whether an update is required.
    fn get_update_info(&self) -> Result<UpdateRequiredInfo>;

    /// Build the updater from the self_update crate.
    fn build_updater(&self, info: &UpdateRequiredInfo) -> Result<Box<dyn ReleaseUpdate>>;

    /// Update the binary. Install if not present, in the case of additional binaries
    /// such as Revela.
    fn update(&self) -> CliTypedResult<String> {
        // Confirm that we need to update.
        let info = self
            .get_update_info()
            .context("Failed to check if we need to update")?;
        if !info.update_required()? {
            return Ok(format!("Already up to date (v{})", info.target_version));
        }

        // Build the updater.
        let updater = self.build_updater(&info)?;

        // Update the binary.
        let result = updater
            .update()
            .map_err(|e| anyhow!("Failed to update {}: {:#}", self.pretty_name(), e))?;

        let message = match result {
            Status::UpToDate(_) => unreachable!("We should have caught this already"),
            Status::Updated(_) => match info.current_version {
                Some(current_version) => format!(
                    "Successfully updated {} from v{} to v{}",
                    self.pretty_name(),
                    current_version,
                    info.target_version
                ),
                None => {
                    format!(
                        "Successfully installed {} v{}",
                        self.pretty_name(),
                        info.target_version
                    )
                },
            },
        };

        Ok(message)
    }
}

/// Information used to determine if an update is required. The versions given to this
/// struct should not have any prefix, it should just be the version. e.g. 2.5.0 rather
/// than velor-cli-v2.5.0.
#[derive(Debug)]
pub struct UpdateRequiredInfo {
    pub current_version: Option<String>,
    pub target_version: String,
}

impl UpdateRequiredInfo {
    pub fn update_required(&self) -> Result<bool> {
        match self.current_version {
            Some(ref current_version) => {
                // ignore ".beta" or ".rc" for version comparison
                // because bump_is_greater only supports comparison between `x.y.z`
                // as a result, `1.0.0.rc1` cannot be updated to `1.0.0.rc2`
                let target_version = if self.target_version.ends_with(".beta") {
                    &self.target_version[0..self.target_version.len() - 5]
                } else if self.target_version.ends_with(".rc") {
                    &self.target_version[0..self.target_version.len() - 3]
                } else {
                    &self.target_version
                };
                bump_is_greater(current_version, target_version).context(
                    "Failed to compare current and latest CLI versions, please update manually",
                )
            },
            None => Ok(true),
        }
    }
}

async fn update_binary<Updater: BinaryUpdater + Sync + Send + 'static>(
    updater: Updater,
) -> CliTypedResult<String> {
    let name = updater.pretty_name();
    if updater.check() {
        let info = tokio::task::spawn_blocking(move || updater.get_update_info())
            .await
            .context(format!("Failed to check {} version", name))??;
        if info.current_version.unwrap_or_default() != info.target_version {
            return Ok(format!("Update is available ({})", info.target_version));
        }

        return Ok(format!("Already up to date ({})", info.target_version));
    }

    tokio::task::spawn_blocking(move || updater.update())
        .await
        .context(format!("Failed to install or update {}", name))?
}
