// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_db::LedgerDb,
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
};
use aptos_logger::info;
use aptos_schemadb::SchemaBatch;
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use std::sync::Arc;

#[derive(Debug)]
pub struct EventStorePruner {
    ledger_db: Arc<LedgerDb>,
}

impl DBSubPruner for EventStorePruner {
    fn name(&self) -> &str {
        "EventStorePruner"
    }

    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let batch = SchemaBatch::new();
        self.ledger_db
            .event_db()
            .prune_events(current_progress, target_version, &batch)?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::EventPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        self.ledger_db.event_db().write_schemas(batch)
    }
}

impl EventStorePruner {
    pub(in crate::pruner) fn new(
        ledger_db: Arc<LedgerDb>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            ledger_db.event_db_raw(),
            &DbMetadataKey::EventPrunerProgress,
            metadata_progress,
        )?;

        let myself = EventStorePruner { ledger_db };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up EventStorePruner."
        );
        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }
}
