// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

mod state_kv_metadata_pruner;
pub(crate) mod state_kv_pruner_manager;
mod state_kv_shard_pruner;

use crate::{
    metrics::{OTHER_TIMERS_SECONDS, PRUNER_VERSIONS},
    pruner::{
        db_pruner::DBPruner,
        state_kv_pruner::{
            state_kv_metadata_pruner::StateKvMetadataPruner,
            state_kv_shard_pruner::StateKvShardPruner,
        },
    },
    state_kv_db::StateKvDb,
};
use anyhow::anyhow;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::info;
use aptos_storage_interface::Result;
use aptos_types::transaction::{AtomicVersion, Version};
use rayon::prelude::*;
use std::{
    cmp::min,
    sync::{atomic::Ordering, Arc},
};

pub const STATE_KV_PRUNER_NAME: &str = "state_kv_pruner";

/// Responsible for pruning state kv db.
pub(crate) struct StateKvPruner {
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    progress: AtomicVersion,

    metadata_pruner: StateKvMetadataPruner,
    // Non-empty iff sharding is enabled.
    shard_pruners: Vec<StateKvShardPruner>,
}

impl DBPruner for StateKvPruner {
    fn name(&self) -> &'static str {
        STATE_KV_PRUNER_NAME
    }

    fn prune(&self, max_versions: usize) -> Result<Version> {
        let _timer = OTHER_TIMERS_SECONDS
            .with_label_values(&["state_kv_pruner__prune"])
            .start_timer();

        let mut progress = self.progress();
        let target_version = self.target_version();

        while progress < target_version {
            let current_batch_target_version =
                min(progress + max_versions as Version, target_version);

            info!(
                progress = progress,
                target_version = current_batch_target_version,
                "Pruning state kv data."
            );
            self.metadata_pruner
                .prune(progress, current_batch_target_version)?;

            THREAD_MANAGER.get_background_pool().install(|| {
                self.shard_pruners.par_iter().try_for_each(|shard_pruner| {
                    shard_pruner
                        .prune(progress, current_batch_target_version)
                        .map_err(|err| {
                            anyhow!(
                                "Failed to prune state kv shard {}: {err}",
                                shard_pruner.shard_id(),
                            )
                        })
                })
            })?;

            progress = current_batch_target_version;
            self.record_progress(progress);
            info!(progress = progress, "Pruning state kv data is done.");
        }

        Ok(target_version)
    }

    fn progress(&self) -> Version {
        self.progress.load(Ordering::SeqCst)
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::SeqCst);
        PRUNER_VERSIONS
            .with_label_values(&["state_kv_pruner", "target"])
            .set(target_version as i64);
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::SeqCst)
    }

    fn record_progress(&self, progress: Version) {
        self.progress.store(progress, Ordering::SeqCst);
        PRUNER_VERSIONS
            .with_label_values(&["state_kv_pruner", "progress"])
            .set(progress as i64);
    }
}

impl StateKvPruner {
    pub fn new(state_kv_db: Arc<StateKvDb>) -> Result<Self> {
        info!(name = STATE_KV_PRUNER_NAME, "Initializing...");

        let metadata_pruner = StateKvMetadataPruner::new(Arc::clone(&state_kv_db));

        let metadata_progress = metadata_pruner.progress()?;

        info!(
            metadata_progress = metadata_progress,
            "Created state kv metadata pruner, start catching up all shards."
        );

        let shard_pruners = if state_kv_db.enabled_sharding() {
            let num_shards = state_kv_db.num_shards();
            let mut shard_pruners = Vec::with_capacity(num_shards);
            for shard_id in 0..num_shards {
                shard_pruners.push(StateKvShardPruner::new(
                    shard_id,
                    state_kv_db.db_shard_arc(shard_id),
                    metadata_progress,
                )?);
            }
            shard_pruners
        } else {
            Vec::new()
        };

        let pruner = StateKvPruner {
            target_version: AtomicVersion::new(metadata_progress),
            progress: AtomicVersion::new(metadata_progress),
            metadata_pruner,
            shard_pruners,
        };

        info!(
            name = pruner.name(),
            progress = metadata_progress,
            "Initialized."
        );

        Ok(pruner)
    }
}
