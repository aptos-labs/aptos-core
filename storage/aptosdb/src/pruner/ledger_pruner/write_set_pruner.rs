// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_db::{LedgerDb, write_set_db::WriteSetDb},
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
};
use aptos_logger::info;
use aptos_schemadb::batch::SchemaBatch;
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use std::sync::Arc;

#[derive(Debug)]
pub struct WriteSetPruner {
    ledger_db: Arc<LedgerDb>,
}

impl DBSubPruner for WriteSetPruner {
    fn name(&self) -> &str {
        "WriteSetPruner"
    }

    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let mut batch = SchemaBatch::new();
        WriteSetDb::prune(current_progress, target_version, &mut batch)?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::WriteSetPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        self.ledger_db.write_set_db().write_schemas(batch)
    }
}

impl WriteSetPruner {
    pub(in crate::pruner) fn new(
        ledger_db: Arc<LedgerDb>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            ledger_db.write_set_db_raw(),
            &DbMetadataKey::WriteSetPrunerProgress,
            metadata_progress,
        )?;

        let myself = WriteSetPruner { ledger_db };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up WriteSetPruner."
        );
        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }
}
