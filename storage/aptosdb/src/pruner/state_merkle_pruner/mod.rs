// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

pub(in crate::pruner) mod generics;
pub(crate) mod leaked_stale_node_cleaner;
mod state_merkle_metadata_pruner;
pub(crate) mod state_merkle_pruner_manager;
mod state_merkle_shard_pruner;
#[cfg(test)]
mod test;

use crate::{
    metrics::{OTHER_TIMERS_SECONDS, PRUNER_VERSIONS},
    pruner::{db_pruner::DBPruner, state_merkle_pruner::generics::MerklePrunerSchema},
    sharded_jmt_merkle_db::ShardedJmtMerkleDb,
};
use anyhow::anyhow;
use aptos_experimental_runtimes::thread_manager::THREAD_MANAGER;
use aptos_jellyfish_merkle::{node_type::NodeKey, StaleNodeIndex};
use aptos_logger::info;
use aptos_metrics_core::TimerHelper;
use aptos_schemadb::{schema::KeyCodec, ReadOptions, DB};
use aptos_storage_interface::Result;
use aptos_types::transaction::{AtomicVersion, Version};
use rayon::prelude::*;
// Re-exported for the native-position merkle pruner.
pub(in crate::pruner) use state_merkle_metadata_pruner::StateMerkleMetadataPruner;
pub(in crate::pruner) use state_merkle_shard_pruner::StateMerkleShardPruner;
use std::{
    marker::PhantomData,
    ops::Deref,
    sync::{atomic::Ordering, Arc},
};

/// Responsible for pruning the state tree.
pub struct StateMerklePruner<M> {
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    /// Overall progress, updated when the whole version is done.
    progress: AtomicVersion,

    metadata_pruner: StateMerkleMetadataPruner<M>,
    // Non-empty iff sharding is enabled.
    shard_pruners: Vec<StateMerkleShardPruner<M>>,

    _phantom: PhantomData<M>,
}

impl<M: MerklePrunerSchema> DBPruner for StateMerklePruner<M>
where
    StaleNodeIndex: KeyCodec<M::StaleIndexSchema>,
{
    fn name(&self) -> &'static str {
        M::name()
    }

    fn prune(&self, batch_size: usize) -> Result<Version> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["state_merkle_pruner__prune"]);
        let mut progress = self.progress();
        let target_version = self.target_version();

        if progress >= target_version {
            return Ok(progress);
        }

        info!(
            name = M::name(),
            current_progress = progress,
            target_version = target_version,
            "Start pruning..."
        );

        while progress < target_version {
            if let Some(target_version_for_this_round) = self
                .metadata_pruner
                .maybe_prune_single_version(progress, target_version)?
            {
                self.prune_shards(progress, target_version_for_this_round, batch_size)?;
                progress = target_version_for_this_round;
                info!(name = M::name(), progress = progress);
                self.record_progress(target_version_for_this_round);
            } else {
                self.prune_shards(progress, target_version, batch_size)?;
                self.record_progress(target_version);
                break;
            }
        }

        info!(name = M::name(), progress = target_version, "Done pruning.");

        Ok(target_version)
    }

    fn progress(&self) -> Version {
        self.progress.load(Ordering::SeqCst)
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::SeqCst);
        PRUNER_VERSIONS
            .with_label_values(&[M::name(), "target"])
            .set(target_version as i64);
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::SeqCst)
    }

    fn record_progress(&self, progress: Version) {
        self.progress.store(progress, Ordering::SeqCst);
        PRUNER_VERSIONS
            .with_label_values(&[M::name(), "progress"])
            .set(progress as i64);
    }
}

impl<M: MerklePrunerSchema> StateMerklePruner<M>
where
    StaleNodeIndex: KeyCodec<M::StaleIndexSchema>,
{
    pub fn new<D: Deref<Target = ShardedJmtMerkleDb>>(state_merkle_db: Arc<D>) -> Result<Self> {
        info!(name = M::name(), "Initializing...");

        let metadata_pruner = StateMerkleMetadataPruner::new(state_merkle_db.metadata_db_arc());
        let metadata_progress = metadata_pruner.progress()?;

        info!(
            metadata_progress = metadata_progress,
            "Created {} metadata pruner, start catching up all shards.",
            M::name(),
        );

        let num_shards = state_merkle_db.num_shards();
        let mut shard_pruners = Vec::with_capacity(num_shards);
        for shard_id in 0..num_shards {
            shard_pruners.push(StateMerkleShardPruner::new(
                shard_id,
                state_merkle_db.db_shard_arc(shard_id),
                metadata_progress,
            )?);
        }

        let pruner = StateMerklePruner {
            target_version: AtomicVersion::new(metadata_progress),
            progress: AtomicVersion::new(metadata_progress),
            metadata_pruner,
            shard_pruners,
            _phantom: PhantomData,
        };

        info!(
            name = M::name(),
            progress = metadata_progress,
            "Initialized."
        );

        Ok(pruner)
    }

    fn prune_shards(
        &self,
        current_progress: Version,
        target_version: Version,
        batch_size: usize,
    ) -> Result<()> {
        THREAD_MANAGER
            .get_background_pool()
            .install(|| {
                self.shard_pruners.par_iter().try_for_each(|shard_pruner| {
                    shard_pruner
                        .prune(current_progress, target_version, batch_size)
                        .map_err(|err| {
                            anyhow!(
                                "Failed to prune {} shard {}: {err}",
                                M::name(),
                                shard_pruner.shard_id(),
                            )
                        })
                })
            })
            .map_err(Into::into)
    }

    pub(in crate::pruner::state_merkle_pruner) fn get_stale_node_indices(
        state_merkle_db_shard: &DB,
        start_version: Version,
        target_version: Version,
        limit: usize,
    ) -> Result<(Vec<StaleNodeIndex>, Option<Version>)> {
        let mut indices = Vec::new();
        let mut read_opts = ReadOptions::default();
        read_opts.fill_cache(false);
        let mut iter = state_merkle_db_shard.iter_with_opts::<M::StaleIndexSchema>(read_opts)?;
        iter.seek(&StaleNodeIndex {
            stale_since_version: start_version,
            node_key: NodeKey::new_empty_path(0),
        })?;

        let mut next_version = None;
        while indices.len() < limit {
            if let Some((index, _)) = iter.next().transpose()? {
                next_version = Some(index.stale_since_version);
                if index.stale_since_version <= target_version {
                    indices.push(index);
                    continue;
                }
            }
            break;
        }

        Ok((indices, next_version))
    }
}
