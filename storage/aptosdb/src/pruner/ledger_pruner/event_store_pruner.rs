// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_db::LedgerDb,
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
};
use aptos_db_indexer::db_indexer::InternalIndexerDB;
use aptos_db_indexer_schemas::{
    metadata::{MetadataKey as IndexerMetadataKey, MetadataValue as IndexerMetadataValue},
    schema::indexer_metadata::InternalIndexerMetadataSchema,
};
use aptos_logger::info;
use aptos_schemadb::SchemaBatch;
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use std::sync::Arc;

#[derive(Debug)]
pub struct EventStorePruner {
    ledger_db: Arc<LedgerDb>,
    internal_indexer_db: Option<InternalIndexerDB>,
}

impl DBSubPruner for EventStorePruner {
    fn name(&self) -> &str {
        "EventStorePruner"
    }

    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let batch = SchemaBatch::new();
        let indexer_deletes = SchemaBatch::new();
        let event_indexer_enabled = self.internal_indexer_db.is_some()
            && self.internal_indexer_db.as_ref().unwrap().event_enabled();
        self.ledger_db.event_db().prune_events(
            current_progress,
            target_version,
            &batch,
            &indexer_deletes,
            self.ledger_db.enable_storage_sharding(),
            event_indexer_enabled,
        )?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::EventPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        if let Some(indexer_db) = &self.internal_indexer_db {
            if indexer_db.event_enabled() {
                indexer_deletes.put::<InternalIndexerMetadataSchema>(
                    &IndexerMetadataKey::EventPrunerProgress,
                    &IndexerMetadataValue::Version(target_version),
                )?;
                indexer_db
                    .get_inner_db_ref()
                    .write_schemas(indexer_deletes)?;
            }
        }
        self.ledger_db.event_db().write_schemas(batch)
    }
}

impl EventStorePruner {
    pub(in crate::pruner) fn new(
        ledger_db: Arc<LedgerDb>,
        metadata_progress: Version,
        internal_indexer_db: Option<InternalIndexerDB>,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            ledger_db.event_db_raw(),
            &DbMetadataKey::EventPrunerProgress,
            metadata_progress,
        )?;

        let myself = EventStorePruner {
            ledger_db,
            internal_indexer_db,
        };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up EventStorePruner."
        );
        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }
}
