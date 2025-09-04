// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    ledger_db::transaction_accumulator_db::TransactionAccumulatorDb,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        transaction_info::TransactionInfoSchema,
    },
    utils::iterators::ExpectContinuousVersions,
};
use aptos_schemadb::{DB, batch::SchemaBatch};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    proof::TransactionInfoWithProof,
    transaction::{TransactionInfo, Version},
};
use std::{path::Path, sync::Arc};

#[derive(Debug)]
pub(crate) struct TransactionInfoDb {
    db: Arc<DB>,
}

impl TransactionInfoDb {
    pub(super) fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    pub(super) fn create_checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        self.db.create_checkpoint(path)
    }

    pub(super) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.db.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionInfoPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(super) fn db(&self) -> &DB {
        &self.db
    }

    pub(crate) fn write_schemas(&self, batch: SchemaBatch) -> Result<()> {
        self.db.write_schemas(batch)
    }
}

impl TransactionInfoDb {
    /// Returns transaction info given the `version`.
    pub(crate) fn get_transaction_info(&self, version: Version) -> Result<TransactionInfo> {
        self.db
            .get::<TransactionInfoSchema>(&version)?
            .ok_or_else(|| {
                AptosDbError::NotFound(format!("No TransactionInfo at version {}", version))
            })
    }

    /// Returns an iterator that yields `num_transaction_infos` transaction infos starting from
    /// `start_version`.
    pub(crate) fn get_transaction_info_iter(
        &self,
        start_version: Version,
        num_transaction_infos: usize,
    ) -> Result<impl Iterator<Item = Result<TransactionInfo>> + '_> {
        let mut iter = self.db.iter::<TransactionInfoSchema>()?;
        iter.seek(&start_version)?;
        iter.expect_continuous_versions(start_version, num_transaction_infos)
    }

    /// Returns transaction info at `version` with proof towards root of ledger at `ledger_version`.
    pub(crate) fn get_transaction_info_with_proof(
        &self,
        version: Version,
        ledger_version: Version,
        transaction_accumulator_db: &TransactionAccumulatorDb,
    ) -> Result<TransactionInfoWithProof> {
        Ok(TransactionInfoWithProof::new(
            transaction_accumulator_db.get_transaction_proof(version, ledger_version)?,
            self.get_transaction_info(version)?,
        ))
    }

    /// Saves transaction inf at `version`.
    pub(crate) fn put_transaction_info(
        version: Version,
        transaction_info: &TransactionInfo,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        batch.put::<TransactionInfoSchema>(&version, transaction_info)
    }

    /// Deletes the transaction info between a range of version in [begin, end)
    pub(crate) fn prune(begin: Version, end: Version, batch: &mut SchemaBatch) -> Result<()> {
        for version in begin..end {
            batch.delete::<TransactionInfoSchema>(&version)?;
        }
        Ok(())
    }
}
