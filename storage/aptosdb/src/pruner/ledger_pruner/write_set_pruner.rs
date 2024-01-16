// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    transaction_store::TransactionStore,
};
use aptos_logger::info;
use aptos_schemadb::{SchemaBatch, DB};
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use std::sync::Arc;

#[derive(Debug)]
pub struct WriteSetPruner {
    transaction_store: Arc<TransactionStore>,
    write_set_db: Arc<DB>,
}

impl DBSubPruner for WriteSetPruner {
    fn name(&self) -> &str {
        "WriteSetPruner"
    }

    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let batch = SchemaBatch::new();
        self.transaction_store
            .prune_write_set(current_progress, target_version, &batch)?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::WriteSetPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        self.write_set_db.write_schemas(batch)
    }
}

impl WriteSetPruner {
    pub(in crate::pruner) fn new(
        transaction_store: Arc<TransactionStore>,
        write_set_db: Arc<DB>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            &write_set_db,
            &DbMetadataKey::WriteSetPrunerProgress,
            metadata_progress,
        )?;

        let myself = WriteSetPruner {
            transaction_store,
            write_set_db,
        };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up WriteSetPruner."
        );
        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }
}
