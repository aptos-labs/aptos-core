// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    TransactionStore,
};
use anyhow::Result;
use aptos_logger::info;
use aptos_schemadb::{SchemaBatch, DB};
use aptos_types::transaction::Version;
use std::sync::Arc;

#[derive(Debug)]
pub struct TransactionInfoPruner {
    transaction_store: Arc<TransactionStore>,
    transaction_info_db: Arc<DB>,
}

impl DBSubPruner for TransactionInfoPruner {
    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let batch = SchemaBatch::new();
        self.transaction_store.prune_transaction_info_schema(
            current_progress,
            target_version,
            &batch,
        )?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionInfoPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        self.transaction_info_db.write_schemas(batch)
    }
}

impl TransactionInfoPruner {
    pub(in crate::pruner) fn new(
        transaction_store: Arc<TransactionStore>,
        transaction_info_db: Arc<DB>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            &transaction_info_db,
            &DbMetadataKey::TransactionInfoPrunerProgress,
            metadata_progress,
        )?;

        let myself = TransactionInfoPruner {
            transaction_store,
            transaction_info_db,
        };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up TransactionInfoPruner."
        );
        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }
}
