// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Sub-pruner wrapper that schedules the native-position value-CF
//! pruner under the umbrella [`super::LedgerPruner`].
//!
//! The wrapper:
//! - Calls the underlying `prune_up_to(horizon)` (which drains the
//!   stale-index across all 16 shards in parallel).
//! - Persists progress via `PositionDb::write_pruner_progress` (writes
//!   to the position metadata DB), so restart does not re-scan rows
//!   that have already been collected.
//!
//! When the native-position subsystem isn't initialized on this node
//! (e.g. validator running pre-activation), the position DB handle is
//! `None` and the wrapper is simply not constructed.

use crate::{
    position_db::PositionDb,
    position_pruner::PositionPruner,
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::db_metadata::DbMetadataKey,
};
use aptos_logger::info;
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use std::sync::Arc;

/// `DBSubPruner` adapter for [`PositionPruner`]. Progress lives in the
/// position metadata DB — same place `PositionCommitProgress` /
/// `PositionShardCommitProgress` live, mirroring how main state's
/// pruner progress sits in `state_kv_metadata_db`.
#[derive(Debug)]
pub(in crate::pruner) struct PositionValuePruner {
    inner: Arc<PositionPruner>,
    position_db: Arc<PositionDb>,
}

impl PositionValuePruner {
    pub(in crate::pruner) fn new(
        position_db: Arc<PositionDb>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            position_db.metadata_db(),
            &DbMetadataKey::PositionPrunerProgress,
            metadata_progress,
        )?;
        let inner = Arc::new(PositionPruner::new(Arc::clone(&position_db)));
        let myself = Self { inner, position_db };
        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up PositionValuePruner."
        );
        myself.prune(progress, metadata_progress)?;
        Ok(myself)
    }
}

impl DBSubPruner for PositionValuePruner {
    fn name(&self) -> &str {
        "PositionValuePruner"
    }

    fn prune(&self, _current_progress: Version, target_version: Version) -> Result<()> {
        self.inner.prune_up_to(target_version)?;
        self.position_db.write_pruner_progress(target_version)
    }
}
