// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        transaction_auxiliary_data::TransactionAuxiliaryDataSchema,
    },
    utils::iterators::ExpectContinuousVersions,
};
use aptos_schemadb::{DB, batch::SchemaBatch};
use aptos_storage_interface::Result;
use aptos_types::transaction::{TransactionAuxiliaryData, Version};
use std::{path::Path, sync::Arc};

#[derive(Debug)]
pub(crate) struct TransactionAuxiliaryDataDb {
    db: Arc<DB>,
}

impl TransactionAuxiliaryDataDb {
    pub(super) fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    pub(super) fn create_checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        self.db.create_checkpoint(path)
    }

    pub(super) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.db.put::<DbMetadataSchema>(
            &DbMetadataKey::TransactionAuxiliaryDataPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(super) fn db(&self) -> &DB {
        &self.db
    }

    pub(crate) fn write_schemas(&self, batch: SchemaBatch) -> Result<()> {
        self.db.write_schemas(batch)
    }

    pub(crate) fn get_transaction_auxiliary_data(
        &self,
        version: Version,
    ) -> Result<Option<TransactionAuxiliaryData>> {
        self.db.get::<TransactionAuxiliaryDataSchema>(&version)
    }

    /// Returns an iterator that yields `num_transaction_infos` transaction infos starting from
    /// `start_version`.
    pub(crate) fn get_transaction_auxiliary_data_iter(
        &self,
        start_version: Version,
        num_transaction_auxiliary_data: usize,
    ) -> Result<impl Iterator<Item = Result<TransactionAuxiliaryData>> + '_> {
        let mut iter = self.db.iter::<TransactionAuxiliaryDataSchema>()?;
        iter.seek(&start_version)?;
        iter.expect_continuous_versions(start_version, num_transaction_auxiliary_data)
    }

    /// Saves transaction inf at `version`.
    pub(crate) fn put_transaction_auxiliary_data(
        version: Version,
        transaction_auxiliary_data: &TransactionAuxiliaryData,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        batch.put::<TransactionAuxiliaryDataSchema>(&version, transaction_auxiliary_data)
    }

    /// Deletes the transaction info between a range of version in [begin, end)
    pub(crate) fn prune(begin: Version, end: Version, batch: &mut SchemaBatch) -> Result<()> {
        for version in begin..end {
            batch.delete::<TransactionAuxiliaryDataSchema>(&version)?;
        }
        Ok(())
    }
}
