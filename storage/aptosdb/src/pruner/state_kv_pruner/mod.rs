// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub(in crate::pruner) mod generics;
#[cfg(test)]
mod position_value_test;
mod state_kv_metadata_pruner;
pub(crate) mod state_kv_pruner_manager;
mod state_kv_shard_pruner;

use crate::{
    metrics::{OTHER_TIMERS_SECONDS, PRUNER_VERSIONS},
    pruner::{
        db_pruner::DBPruner,
        state_kv_pruner::{
            generics::StateValuePrunerSchema, state_kv_metadata_pruner::StateKvMetadataPruner,
            state_kv_shard_pruner::StateKvShardPruner,
        },
    },
    sharded_kv_db::ShardedKvDb,
};
use anyhow::anyhow;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_logger::info;
use aptos_metrics_core::TimerHelper;
use aptos_schemadb::schema::SeekKeyCodec;
use aptos_storage_interface::Result;
use aptos_types::{
    state_store::NUM_STATE_SHARDS,
    transaction::{AtomicVersion, Version},
};
use rayon::prelude::*;
use std::{
    cmp::min,
    marker::PhantomData,
    ops::Deref,
    sync::{atomic::Ordering, Arc},
};

/// Responsible for pruning state value data (main-state cold/hot or position).
pub(crate) struct StateKvPruner<S> {
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    progress: AtomicVersion,

    metadata_pruner: StateKvMetadataPruner<S>,
    shard_pruners: Vec<StateKvShardPruner<S>>,

    _phantom: PhantomData<S>,
}

impl<S: StateValuePrunerSchema> DBPruner for StateKvPruner<S>
where
    Version: SeekKeyCodec<S::StaleIndexSchema>,
{
    fn name(&self) -> &'static str {
        S::name()
    }

    fn prune(&self, max_versions: usize) -> Result<Version> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&[&format!("{}__prune", S::name())]);

        let mut progress = self.progress();
        let target_version = self.target_version();

        while progress < target_version {
            let current_batch_target_version =
                min(progress + max_versions as Version, target_version);

            info!(
                progress = progress,
                target_version = current_batch_target_version,
                name = S::name(),
                "Pruning state kv data."
            );
            self.metadata_pruner.prune(current_batch_target_version)?;

            THREAD_MANAGER.get_background_pool().install(|| {
                self.shard_pruners.par_iter().try_for_each(|shard_pruner| {
                    shard_pruner
                        .prune(progress, current_batch_target_version)
                        .map_err(|err| {
                            anyhow!(
                                "Failed to prune {} shard {}: {err}",
                                S::name(),
                                shard_pruner.shard_id(),
                            )
                        })
                })
            })?;

            progress = current_batch_target_version;
            self.record_progress(progress);
            info!(
                progress = progress,
                name = S::name(),
                "Pruning state kv data is done."
            );
        }

        Ok(target_version)
    }

    fn progress(&self) -> Version {
        self.progress.load(Ordering::SeqCst)
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::SeqCst);
        PRUNER_VERSIONS
            .with_label_values(&[S::name(), "target"])
            .set(target_version as i64);
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::SeqCst)
    }

    fn record_progress(&self, progress: Version) {
        self.progress.store(progress, Ordering::SeqCst);
        PRUNER_VERSIONS
            .with_label_values(&[S::name(), "progress"])
            .set(progress as i64);
    }
}

impl<S: StateValuePrunerSchema> StateKvPruner<S>
where
    Version: SeekKeyCodec<S::StaleIndexSchema>,
{
    pub(in crate::pruner) fn new<D: Deref<Target = ShardedKvDb>>(
        state_kv_db: Arc<D>,
    ) -> Result<Self> {
        info!(name = S::name(), "Initializing...");

        let metadata_pruner = StateKvMetadataPruner::new(Arc::clone(state_kv_db.metadata_db()));

        let metadata_progress = metadata_pruner.progress()?;

        info!(
            metadata_progress = metadata_progress,
            name = S::name(),
            "Created state kv metadata pruner, start catching up all shards."
        );

        let mut shard_pruners = Vec::with_capacity(NUM_STATE_SHARDS);
        for shard_id in 0..NUM_STATE_SHARDS {
            shard_pruners.push(StateKvShardPruner::new(
                shard_id,
                Arc::clone(state_kv_db.shard(shard_id)),
                metadata_progress,
            )?);
        }

        let pruner = StateKvPruner {
            target_version: AtomicVersion::new(metadata_progress),
            progress: AtomicVersion::new(metadata_progress),
            metadata_pruner,
            shard_pruners,
            _phantom: PhantomData,
        };

        info!(
            name = S::name(),
            progress = metadata_progress,
            "Initialized."
        );

        Ok(pruner)
    }
}
