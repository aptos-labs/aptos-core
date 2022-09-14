// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::pruner::db_pruner::DBPruner;
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
    type Pruner: DBPruner;

    fn is_pruner_enabled(&self) -> bool;

    fn get_prune_window(&self) -> Version;

    fn get_min_viable_version(&self) -> Version;

    fn get_min_readable_version(&self) -> Version;

    /// Sets pruner target version when necessary.
    fn maybe_set_pruner_target_db_version(&self, latest_version: Version);

    fn set_pruner_target_db_version(&self, latest_version: Version);

    fn pruner(&self) -> &Self::Pruner;

    /// (For tests only.) Notifies the worker thread and waits for it to finish its job by polling
    /// an internal counter.
    #[cfg(test)]
    fn wake_and_wait_pruner(&self, latest_version: Version) -> anyhow::Result<()> {
        self.maybe_set_pruner_target_db_version(latest_version);
        self.wait_for_pruner()
    }

    #[cfg(test)]
    fn wait_for_pruner(&self) -> anyhow::Result<()> {
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
            if !self.pruner().is_pruning_pending() {
                return Ok(());
            }
            sleep(Duration::from_millis(1));
        }
        anyhow::bail!("Timeout waiting for pruner worker.");
    }
}
