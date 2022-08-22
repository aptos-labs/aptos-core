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
    fn is_pruner_enabled(&self) -> bool;

    fn get_pruner_window(&self) -> Version;

    fn get_min_viable_version(&self) -> Version;

    fn get_min_readable_version(&self) -> Version;

    /// Sets pruner target version when necessary.
    fn maybe_set_pruner_target_db_version(&self, latest_version: Version);

    fn set_pruner_target_db_version(&self, latest_version: Version);

    /// (For tests only.) Notifies the worker thread and waits for it to finish its job by polling
    /// an internal counter.
    #[cfg(test)]
    fn wake_and_wait_pruner(&self, latest_version: Version) -> anyhow::Result<()>;
}
