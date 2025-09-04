// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides `Pruner` which manages a thread pruning old data in the background and is
//! meant to be triggered by other threads as they commit new data to the DB.

use crate::{
    metrics::{PRUNER_BATCH_SIZE, PRUNER_VERSIONS, PRUNER_WINDOW},
    pruner::{
        pruner_manager::PrunerManager,
        pruner_utils,
        pruner_worker::PrunerWorker,
        state_merkle_pruner::{StateMerklePruner, generics::StaleNodeIndexSchemaTrait},
    },
    state_merkle_db::StateMerkleDb,
};
use aptos_config::config::StateMerklePrunerConfig;
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_schemadb::schema::KeyCodec;
use aptos_storage_interface::Result;
use aptos_types::transaction::{AtomicVersion, Version};
use std::{
    marker::PhantomData,
    sync::{Arc, atomic::Ordering},
};

/// The `Pruner` is meant to be part of a `AptosDB` instance and runs in the background to prune old
/// data.
///
/// If the state pruner is enabled, it creates a worker thread on construction and joins it on
/// destruction. When destructed, it quits the worker thread eagerly without waiting for all
/// pending work to be done.
pub struct StateMerklePrunerManager<S: StaleNodeIndexSchemaTrait>
where
    StaleNodeIndex: KeyCodec<S>,
{
    state_merkle_db: Arc<StateMerkleDb>,
    /// DB version window, which dictates how many versions of state merkle data to keep.
    prune_window: Version,
    /// It is None iff the pruner is not enabled.
    pruner_worker: Option<PrunerWorker>,
    /// The minimal readable version for the state merkle data.
    min_readable_version: AtomicVersion,

    _phantom: PhantomData<S>,
}

impl<S: StaleNodeIndexSchemaTrait> PrunerManager for StateMerklePrunerManager<S>
where
    StaleNodeIndex: KeyCodec<S>,
{
    type Pruner = StateMerklePruner<S>;

    fn is_pruner_enabled(&self) -> bool {
        self.pruner_worker.is_some()
    }

    fn get_prune_window(&self) -> Version {
        self.prune_window
    }

    fn get_min_readable_version(&self) -> Version {
        self.min_readable_version.load(Ordering::SeqCst)
    }

    /// Sets pruner target version when necessary.
    fn maybe_set_pruner_target_db_version(&self, latest_version: Version) {
        let min_readable_version = self.get_min_readable_version();
        if self.is_pruner_enabled() && latest_version >= min_readable_version + self.prune_window {
            self.set_pruner_target_db_version(latest_version);
        }
    }

    fn save_min_readable_version(&self, min_readable_version: Version) -> Result<()> {
        self.min_readable_version
            .store(min_readable_version, Ordering::SeqCst);

        PRUNER_VERSIONS
            .with_label_values(&[S::name(), "min_readable"])
            .set(min_readable_version as i64);

        self.state_merkle_db
            .write_pruner_progress(&S::progress_metadata_key(None), min_readable_version)
    }

    fn is_pruning_pending(&self) -> bool {
        self.pruner_worker
            .as_ref()
            .is_some_and(|w| w.is_pruning_pending())
    }

    #[cfg(test)]
    fn set_worker_target_version(&self, target_version: Version) {
        self.pruner_worker
            .as_ref()
            .unwrap()
            .set_target_db_version(target_version);
    }
}

impl<S: StaleNodeIndexSchemaTrait> StateMerklePrunerManager<S>
where
    StaleNodeIndex: KeyCodec<S>,
{
    /// Creates a worker thread that waits on a channel for pruning commands.
    pub fn new(
        state_merkle_db: Arc<StateMerkleDb>,
        state_merkle_pruner_config: StateMerklePrunerConfig,
    ) -> Self {
        let pruner_worker = if state_merkle_pruner_config.enable {
            Some(Self::init_pruner(
                Arc::clone(&state_merkle_db),
                state_merkle_pruner_config,
            ))
        } else {
            None
        };

        let min_readable_version = pruner_utils::get_state_merkle_pruner_progress(&state_merkle_db)
            .expect("Must succeed.");

        PRUNER_VERSIONS
            .with_label_values(&[S::name(), "min_readable"])
            .set(min_readable_version as i64);

        Self {
            state_merkle_db,
            prune_window: state_merkle_pruner_config.prune_window,
            pruner_worker,
            min_readable_version: AtomicVersion::new(min_readable_version),
            _phantom: PhantomData,
        }
    }

    fn init_pruner(
        state_merkle_db: Arc<StateMerkleDb>,
        state_merkle_pruner_config: StateMerklePrunerConfig,
    ) -> PrunerWorker {
        let pruner = Arc::new(
            StateMerklePruner::<S>::new(Arc::clone(&state_merkle_db))
                .expect("Failed to create state merkle pruner."),
        );

        PRUNER_WINDOW
            .with_label_values(&[S::name()])
            .set(state_merkle_pruner_config.prune_window as i64);

        PRUNER_BATCH_SIZE
            .with_label_values(&[S::name()])
            .set(state_merkle_pruner_config.batch_size as i64);

        PrunerWorker::new(
            pruner,
            state_merkle_pruner_config.batch_size,
            "state_merkle",
        )
    }

    fn set_pruner_target_db_version(&self, latest_version: Version) {
        assert!(self.pruner_worker.is_some());

        let min_readable_version = latest_version.saturating_sub(self.prune_window);
        self.min_readable_version
            .store(min_readable_version, Ordering::SeqCst);

        PRUNER_VERSIONS
            .with_label_values(&[S::name(), "min_readable"])
            .set(min_readable_version as i64);

        self.pruner_worker
            .as_ref()
            .unwrap()
            .set_target_db_version(min_readable_version);
    }
}
