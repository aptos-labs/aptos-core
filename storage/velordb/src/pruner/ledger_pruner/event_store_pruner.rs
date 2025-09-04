// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_db::LedgerDb,
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
};
use velor_db_indexer::db_indexer::InternalIndexerDB;
use velor_db_indexer_schemas::{
    metadata::{MetadataKey as IndexerMetadataKey, MetadataValue as IndexerMetadataValue},
    schema::indexer_metadata::InternalIndexerMetadataSchema,
};
use velor_logger::info;
use velor_schemadb::batch::SchemaBatch;
use velor_storage_interface::Result;
use velor_types::transaction::Version;
use std::sync::Arc;

#[derive(Debug)]
pub struct EventStorePruner {
    ledger_db: Arc<LedgerDb>,
    internal_indexer_db: Option<InternalIndexerDB>,
}

impl EventStorePruner {
    fn expect_indexer_db(&self) -> &InternalIndexerDB {
        self.internal_indexer_db
            .as_ref()
            .expect("internal indexer not enabled")
    }

    fn indexer_db(&self) -> Option<&InternalIndexerDB> {
        self.internal_indexer_db.as_ref()
    }
}

impl DBSubPruner for EventStorePruner {
    fn name(&self) -> &str {
        "EventStorePruner"
    }

    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let mut batch = SchemaBatch::new();
        let mut indexer_batch = None;

        let indices_batch = if let Some(indexer_db) = self.indexer_db() {
            if indexer_db.event_enabled() {
                indexer_batch = Some(SchemaBatch::new());
            }
            indexer_batch.as_mut()
        } else {
            Some(&mut batch)
        };
        let num_events_per_version = self.ledger_db.event_db().prune_event_indices(
            current_progress,
            target_version,
            indices_batch,
        )?;
        self.ledger_db.event_db().prune_events(
            num_events_per_version,
            current_progress,
            target_version,
            &mut batch,
        )?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::EventPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;

        if let Some(mut indexer_batch) = indexer_batch {
            indexer_batch.put::<InternalIndexerMetadataSchema>(
                &IndexerMetadataKey::EventPrunerProgress,
                &IndexerMetadataValue::Version(target_version),
            )?;
            self.expect_indexer_db()
                .get_inner_db_ref()
                .write_schemas(indexer_batch)?;
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
