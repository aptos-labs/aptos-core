// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_metadata::DbMetadataSchema,
    metrics::PRUNER_VERSIONS,
    pruner::{db_pruner::DBPruner, state_store::state_value_pruner::StateValuePruner},
    schema::db_metadata::{DbMetadataKey, DbMetadataValue},
    state_kv_db::StateKvDb,
};
use anyhow::Result;
use aptos_schemadb::SchemaBatch;
use aptos_types::transaction::{AtomicVersion, Version};
use std::sync::{atomic::Ordering, Arc};

pub const STATE_KV_PRUNER_NAME: &str = "state_kv_pruner";

/// Responsible for pruning state kv db.
pub(crate) struct StateKvPruner {
    state_kv_db: Arc<StateKvDb>,
    /// Keeps track of the target version that the pruner needs to achieve.
    target_version: AtomicVersion,
    progress: AtomicVersion,
    state_value_pruner: Arc<StateValuePruner>,
}

impl DBPruner for StateKvPruner {
    fn name(&self) -> &'static str {
        STATE_KV_PRUNER_NAME
    }

    fn prune(&self, max_versions: usize) -> Result<Version> {
        if !self.is_pruning_pending() {
            return Ok(self.progress());
        }

        let mut db_batch = SchemaBatch::new();
        let current_target_version = self.prune_inner(max_versions, &mut db_batch)?;
        self.save_progress(current_target_version, &db_batch)?;
        self.state_kv_db.commit_raw_batch(db_batch)?;
        self.record_progress(current_target_version);

        Ok(current_target_version)
    }

    fn initialize_min_readable_version(&self) -> anyhow::Result<Version> {
        Ok(self
            .state_kv_db
            .metadata_db()
            .get::<DbMetadataSchema>(&DbMetadataKey::StateKvPrunerProgress)?
            .map_or(0, |v| v.expect_version()))
    }

    fn progress(&self) -> Version {
        self.progress.load(Ordering::SeqCst)
    }

    fn set_target_version(&self, target_version: Version) {
        self.target_version.store(target_version, Ordering::Relaxed);
        PRUNER_VERSIONS
            .with_label_values(&["state_kv_pruner", "target"])
            .set(target_version as i64);
    }

    fn target_version(&self) -> Version {
        self.target_version.load(Ordering::Relaxed)
    }

    fn record_progress(&self, min_readable_version: Version) {
        self.progress.store(min_readable_version, Ordering::Relaxed);
        PRUNER_VERSIONS
            .with_label_values(&["state_kv_pruner", "progress"])
            .set(min_readable_version as i64);
    }
}

impl StateKvPruner {
    pub fn new(state_kv_db: Arc<StateKvDb>) -> Self {
        let pruner = StateKvPruner {
            state_kv_db: Arc::clone(&state_kv_db),
            target_version: AtomicVersion::new(0),
            progress: AtomicVersion::new(0),
            state_value_pruner: Arc::new(StateValuePruner::new(state_kv_db)),
        };
        pruner.initialize();
        pruner
    }

    fn prune_inner(
        &self,
        max_versions: usize,
        db_batch: &mut SchemaBatch,
    ) -> anyhow::Result<Version> {
        let progress = self.progress();

        let current_target_version = self.get_current_batch_target(max_versions as Version);
        if current_target_version < progress {
            return Ok(progress);
        }

        self.state_value_pruner
            .prune(db_batch, progress, current_target_version)?;

        Ok(current_target_version)
    }

    fn save_progress(&self, version: Version, batch: &SchemaBatch) -> Result<()> {
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::StateKvPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }
}
