// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pruner::db_pruner::DBPruner;
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;

/// This module provides `Pruner` which manages a thread pruning old data in the background and is
/// meant to be triggered by other threads as they commit new data to the DB.

/// The `PrunerManager` is meant to be part of a `AptosDB` instance and runs in the background to
/// prune old data.
///
/// If the pruner is enabled. It creates a worker thread on construction and joins it on
/// destruction. When destructed, it quits the worker thread eagerly without waiting for
/// all pending work to be done.
pub trait PrunerManager: Sync {
    type Pruner: DBPruner;

    fn is_pruner_enabled(&self) -> bool;

    fn get_prune_window(&self) -> Version;

    fn get_min_viable_version(&self) -> Version {
        unimplemented!()
    }

    fn get_min_readable_version(&self) -> Version;

    /// Sets pruner target version when necessary.
    fn maybe_set_pruner_target_db_version(&self, latest_version: Version);

    // Only used at the end of fast sync to store the min_readable_version to db and update the
    // in memory progress.
    fn save_min_readable_version(&self, min_readable_version: Version) -> Result<()>;

    #[allow(unused)]
    fn is_pruning_pending(&self) -> bool;

    /// (For tests only.) Notifies the worker thread and waits for it to finish its job by polling
    /// an internal counter.
    #[cfg(test)]
    fn wake_and_wait_pruner(&self, latest_version: Version) -> Result<()> {
        self.maybe_set_pruner_target_db_version(latest_version);
        self.wait_for_pruner()
    }

    #[cfg(test)]
    fn wait_for_pruner(&self) -> Result<()> {
        use aptos_storage_interface::{AptosDbError, db_other_bail};
        use std::{
            thread::sleep,
            time::{Duration, Instant},
        };

        if !self.is_pruner_enabled() {
            return Ok(());
        }

        // Assuming no big pruning chunks will be issued by a test.
        const TIMEOUT: Duration = Duration::from_secs(60);
        let end = Instant::now() + TIMEOUT;

        while Instant::now() < end {
            if !self.is_pruning_pending() {
                return Ok(());
            }
            sleep(Duration::from_millis(1));
        }
        db_other_bail!("Timeout waiting for pruner worker.");
    }

    #[cfg(test)]
    fn set_worker_target_version(&self, target_version: Version);
}
