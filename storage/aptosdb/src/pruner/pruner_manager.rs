// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_types::transaction::Version;
use std::fmt::Debug;

/// This module provides `Pruner` which manages a thread pruning old data in the background and is
/// meant to be triggered by other threads as they commit new data to the DB.

/// The `PrunerManager` is meant to be part of a `AptosDB` instance and runs in the background to
/// prune old data.
///
/// It creates a worker thread on construction and joins it on destruction. When destructed, it
/// quits the worker thread eagerly without waiting for all pending work to be done.
pub trait PrunerManager: Debug + Sync {
    fn get_pruner_window(&self) -> Option<Version>;

    fn get_min_readable_version(&self) -> Option<Version>;

    /// Sends pruning command to the worker thread when necessary.
    fn maybe_wake_pruner(&self, latest_version: Version);

    fn wake_pruner(&self, latest_version: Version);

    /// (For tests only.) Notifies the worker thread and waits for it to finish its job by polling
    /// an internal counter.
    #[cfg(test)]
    fn wake_and_wait_pruner(&self, latest_version: Version) -> anyhow::Result<()>;

    /// (For tests only.) Ensure a pruner is disabled.
    #[cfg(test)]
    fn ensure_disabled(&self) -> anyhow::Result<()>;

    /// (For tests only.) Updates the minimal readable version kept by pruner.
    #[cfg(test)]
    fn testonly_update_min_version(&mut self, version: Option<Version>);
}
