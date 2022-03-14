// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::Version;

/// Defines the trait for pruner for different DB
pub trait DBPruner {
    /// Initialize the pruner and record the least readable version
    fn initialize(&self);
    /// Performs the actual pruning, a target version is passed, which is the target the pruner
    /// tries to prune.
    fn prune(&self, max_versions: usize) -> anyhow::Result<Version>;
    /// Initializes the least readable version stored in underlying DB storage
    fn initialize_least_readable_version(&self) -> anyhow::Result<Version>;
    /// Returns the least readable version stores in the DB pruner
    fn least_readable_version(&self) -> Version;
    /// Sets the target version for the pruner
    fn set_target_version(&self, target_version: Version);
    /// Returns the target version for the DB pruner
    fn target_version(&self) -> Version;
    /// Records the current progress of the pruner by updating the least readable version
    fn record_progress(&self, least_readable_version: Version);
    /// True if there is pruning work pending to be done
    fn is_pruning_pending(&self) -> bool;
}
