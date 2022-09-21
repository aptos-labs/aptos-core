// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Context, Result};
use aptos_logger::info;
use aptos_types::transaction::Version;
use std::cmp::min;

/// Defines the trait for pruner for different DB
pub trait DBPruner: Send + Sync {
    /// Find out the first undeleted item in the stale node index.
    fn initialize(&self) {
        let min_readable_version = self
            .initialize_min_readable_version()
            .context(self.name())
            .expect("Pruner failed to initialize.");
        info!(
            min_readable_version = min_readable_version,
            "{} initialized.",
            self.name()
        );
        self.record_progress(min_readable_version);
    }

    fn name(&self) -> &'static str;

    /// Performs the actual pruning, a target version is passed, which is the target the pruner
    /// tries to prune.
    fn prune(&self, batch_size: usize) -> Result<Version>;

    /// Initializes the least readable version stored in underlying DB storage
    fn initialize_min_readable_version(&self) -> Result<Version>;

    /// Returns the least readable version stores in the DB pruner
    fn min_readable_version(&self) -> Version;

    /// Sets the target version for the pruner
    fn set_target_version(&self, target_version: Version);

    /// Returns the target version for the DB pruner
    fn target_version(&self) -> Version;

    /// Returns the target version for the current pruning round - this might be different from the
    /// target_version() because we need to keep max_version in account.
    fn get_current_batch_target(&self, max_versions: Version) -> Version {
        // Current target version  might be less than the target version to ensure we don't prune
        // more than max_version in one go.
        min(
            self.min_readable_version() + max_versions as u64,
            self.target_version(),
        )
    }
    /// Records the current progress of the pruner by updating the least readable version
    fn record_progress(&self, min_readable_version: Version);

    /// True if there is pruning work pending to be done
    fn is_pruning_pending(&self) -> bool {
        self.target_version() > self.min_readable_version()
    }

    /// (For tests only.) Updates the minimal readable version kept by pruner.
    fn testonly_update_min_version(&self, version: Version);
}
