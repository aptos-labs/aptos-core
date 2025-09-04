// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_storage_interface::Result;
use velor_types::transaction::Version;
use std::cmp::min;

/// Defines the trait for pruner for different DB
pub trait DBPruner: Send + Sync {
    fn name(&self) -> &'static str;

    /// Performs the actual pruning, a target version is passed, which is the target the pruner
    /// tries to prune.
    fn prune(&self, batch_size: usize) -> Result<Version>;

    /// Returns the progress of the pruner.
    fn progress(&self) -> Version;

    /// Sets the target version for the pruner
    fn set_target_version(&self, target_version: Version);

    /// Returns the target version for the DB pruner
    fn target_version(&self) -> Version;

    /// Returns the target version for the current pruning round - this might be different from the
    /// target_version() because we need to keep max_version in account.
    #[allow(unused)]
    fn get_current_batch_target(&self, max_versions: Version) -> Version {
        // Current target version  might be less than the target version to ensure we don't prune
        // more than max_version in one go.
        min(self.progress() + max_versions, self.target_version())
    }
    /// Records the current progress of the pruner by updating the least readable version
    fn record_progress(&self, min_readable_version: Version);

    /// True if there is pruning work pending to be done
    fn is_pruning_pending(&self) -> bool {
        self.target_version() > self.progress()
    }
}
