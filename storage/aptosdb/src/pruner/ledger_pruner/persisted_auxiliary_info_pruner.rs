// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_db::{LedgerDb, persisted_auxiliary_info_db::PersistedAuxiliaryInfoDb},
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
};
use aptos_logger::info;
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use std::sync::Arc;

#[derive(Debug)]
pub struct PersistedAuxiliaryInfoPruner {
    ledger_db: Arc<LedgerDb>,
}

impl DBSubPruner for PersistedAuxiliaryInfoPruner {
    fn name(&self) -> &str {
        "PersistedAuxiliaryInfoPruner"
    }

    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let mut batch = SchemaBatch::new();
        PersistedAuxiliaryInfoDb::prune(current_progress, target_version, &mut batch)?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::PersistedAuxiliaryInfoPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        self.ledger_db
            .persisted_auxiliary_info_db()
            .write_schemas(batch)
    }
}

impl PersistedAuxiliaryInfoPruner {
    pub(in crate::pruner) fn new(
        ledger_db: Arc<LedgerDb>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            ledger_db.persisted_auxiliary_info_db_raw(),
            &DbMetadataKey::PersistedAuxiliaryInfoPrunerProgress,
            metadata_progress,
        )?;

        let myself = PersistedAuxiliaryInfoPruner { ledger_db };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up PersistedAuxiliaryInfoPruner."
        );
        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }
}
