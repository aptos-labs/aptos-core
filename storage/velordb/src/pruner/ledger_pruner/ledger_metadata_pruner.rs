// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::schema::{
    db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
    version_data::VersionDataSchema,
};
use velor_schemadb::{batch::SchemaBatch, DB};
use velor_storage_interface::{VelorDbError, Result};
use velor_types::transaction::Version;
use std::sync::Arc;

#[derive(Debug)]
pub struct LedgerMetadataPruner {
    ledger_metadata_db: Arc<DB>,
}

impl LedgerMetadataPruner {
    pub(in crate::pruner) fn new(ledger_metadata_db: Arc<DB>) -> Result<Self> {
        if let Some(v) =
            ledger_metadata_db.get::<DbMetadataSchema>(&DbMetadataKey::LedgerPrunerProgress)?
        {
            v.expect_version();
        } else {
            // NOTE: I **think** all db should have the LedgerPrunerProgress. Have a fallback path
            // here in case the database was super old before we introducing this progress counter.
            let mut iter = ledger_metadata_db.iter::<VersionDataSchema>()?;
            iter.seek_to_first();
            let version = match iter.next().transpose()? {
                Some((version, _)) => version,
                None => 0,
            };
            ledger_metadata_db.put::<DbMetadataSchema>(
                &DbMetadataKey::LedgerPrunerProgress,
                &DbMetadataValue::Version(version),
            )?;
        }

        Ok(LedgerMetadataPruner { ledger_metadata_db })
    }

    pub(in crate::pruner) fn prune(
        &self,
        current_progress: Version,
        target_version: Version,
    ) -> Result<()> {
        let mut batch = SchemaBatch::new();
        for version in current_progress..target_version {
            batch.delete::<VersionDataSchema>(&version)?;
        }
        batch.put::<DbMetadataSchema>(
            &DbMetadataKey::LedgerPrunerProgress,
            &DbMetadataValue::Version(target_version),
        )?;
        self.ledger_metadata_db.write_schemas(batch)
    }

    pub(in crate::pruner) fn progress(&self) -> Result<Version> {
        self.ledger_metadata_db
            .get::<DbMetadataSchema>(&DbMetadataKey::LedgerPrunerProgress)?
            .map(|v| v.expect_version())
            .ok_or_else(|| VelorDbError::Other("LedgerPrunerProgress cannot be None.".to_string()))
    }
}
