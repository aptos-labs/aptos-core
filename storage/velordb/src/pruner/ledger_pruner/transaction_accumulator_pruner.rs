// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_db::{transaction_accumulator_db::TransactionAccumulatorDb, LedgerDb},
    pruner::{db_sub_pruner::DBSubPruner, pruner_utils::get_or_initialize_subpruner_progress},
    schema::db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
};
use velor_logger::info;
use velor_schemadb::batch::SchemaBatch;
use velor_storage_interface::Result;
use velor_types::transaction::Version;
use std::sync::Arc;

#[derive(Debug)]
pub struct TransactionAccumulatorPruner {
    ledger_db: Arc<LedgerDb>,
}

impl DBSubPruner for TransactionAccumulatorPruner {
    fn name(&self) -> &str {
        "TransactionAccumulatorPruner"
    }

    fn prune(&self, current_progress: Version, target_version: Version) -> Result<()> {
        let mut batch = SchemaBatch::new();
        TransactionAccumulatorDb::prune(current_progress, target_version, &mut batch)?;
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionAccumulatorPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        self.ledger_db
            .transaction_accumulator_db()
            .write_schemas(batch)
    }
}

impl TransactionAccumulatorPruner {
    pub(in crate::pruner) fn new(
        ledger_db: Arc<LedgerDb>,
        metadata_progress: Version,
    ) -> Result<Self> {
        let progress = get_or_initialize_subpruner_progress(
            ledger_db.transaction_accumulator_db_raw(),
            &DbMetadataKey::TransactionAccumulatorPrunerProgress,
            metadata_progress,
        )?;

        let myself = TransactionAccumulatorPruner { ledger_db };

        info!(
            progress = progress,
            metadata_progress = metadata_progress,
            "Catching up TransactionAccumulatorPruner."
        );
        myself.prune(progress, metadata_progress)?;

        Ok(myself)
    }
}
