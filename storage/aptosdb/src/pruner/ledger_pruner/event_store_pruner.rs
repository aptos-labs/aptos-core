// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    EventStore,
};
use anyhow::Result;
use aptos_logger::info;
use aptos_schemadb::{SchemaBatch, DB};
use aptos_types::transaction::Version;
use std::sync::Arc;

#[derive(Debug)]
pub struct EventStorePruner {
    event_store: Arc<EventStore>,
    event_db: Arc<DB>,
}

impl DBSubPruner for EventStorePruner {
    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let batch = SchemaBatch::new();
        self.event_store
            .prune_events(current_progress, target_version, &batch)?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::EventPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        self.event_db.write_schemas(batch)
    }
}

impl EventStorePruner {
    pub(in crate::pruner) fn new(
        event_store: Arc<EventStore>,
        event_db: Arc<DB>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            &event_db,
            &DbMetadataKey::EventPrunerProgress,
            metadata_progress,
        )?;

        let myself = EventStorePruner {
            event_store,
            event_db,
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
