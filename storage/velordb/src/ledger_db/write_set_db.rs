// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::OTHER_TIMERS_SECONDS,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        write_set::WriteSetSchema,
    },
    utils::iterators::ExpectContinuousVersions,
};
use velor_metrics_core::TimerHelper;
use velor_schemadb::{
    batch::{SchemaBatch, WriteBatch},
    DB,
};
use velor_storage_interface::{db_ensure as ensure, VelorDbError, Result};
use velor_types::{
    transaction::{TransactionOutput, Version},
    write_set::WriteSet,
};
use rayon::prelude::*;
use std::{path::Path, sync::Arc};

#[derive(Debug)]
pub(crate) struct WriteSetDb {
    db: Arc<DB>,
}

impl WriteSetDb {
    pub(super) fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    pub(super) fn create_checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        self.db.create_checkpoint(path)
    }

    pub(super) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.db.put::<DbMetadataSchema>(
            &DbMetadataKey::WriteSetPrunerProgress,
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

impl WriteSetDb {
    /// Returns executed transaction vm output given the `version`.
    pub(crate) fn get_write_set(&self, version: Version) -> Result<WriteSet> {
        self.db
            .get::<WriteSetSchema>(&version)?
            .ok_or_else(|| VelorDbError::NotFound(format!("WriteSet at version {}", version)))
    }

    /// Returns an iterator that yields `num_transactions` write sets starting from `start_version`.
    pub(crate) fn get_write_set_iter(
        &self,
        start_version: Version,
        num_transactions: usize,
    ) -> Result<impl Iterator<Item = Result<WriteSet>> + '_> {
        let mut iter = self.db.iter::<WriteSetSchema>()?;
        iter.seek(&start_version)?;
        iter.expect_continuous_versions(start_version, num_transactions)
    }

    /// Returns write sets in `[begin_version, end_version)` half-open range.
    ///
    /// N.b. an empty `Vec` is returned when `begin_version == end_version`.
    pub(crate) fn get_write_sets(
        &self,
        begin_version: Version,
        end_version: Version,
    ) -> Result<Vec<WriteSet>> {
        if begin_version == end_version {
            return Ok(Vec::new());
        }
        ensure!(
            begin_version < end_version,
            "begin_version {} >= end_version {}",
            begin_version,
            end_version
        );

        let mut iter = self.db.iter::<WriteSetSchema>()?;
        iter.seek(&begin_version)?;

        let mut ret = Vec::with_capacity((end_version - begin_version) as usize);
        for current_version in begin_version..end_version {
            let (version, write_set) = iter.next().transpose()?.ok_or_else(|| {
                VelorDbError::NotFound(format!("Write set missing for version {}", current_version))
            })?;
            ensure!(
                version == current_version,
                "Write set missing for version {}, got version {}",
                current_version,
                version,
            );
            ret.push(write_set);
        }

        Ok(ret)
    }

    /// Commits write sets starting from `first_version` to the database.
    pub(crate) fn commit_write_sets(
        &self,
        first_version: Version,
        transaction_outputs: &[TransactionOutput],
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_write_sets"]);

        let chunk_size = transaction_outputs.len() / 4 + 1;
        let batches = transaction_outputs
            .par_chunks(chunk_size)
            .enumerate()
            .map(|(chunk_idx, chunk)| {
                let mut batch = self.db().new_native_batch();
                let chunk_first_version = first_version + (chunk_idx * chunk_size) as Version;

                chunk.iter().enumerate().try_for_each(|(i, txn_out)| {
                    Self::put_write_set(
                        chunk_first_version + i as Version,
                        txn_out.write_set(),
                        &mut batch,
                    )
                })?;
                Ok(batch)
            })
            .collect::<Result<Vec<_>>>()?;

        {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_write_sets___commit"]);
            for batch in batches {
                self.db().write_schemas(batch)?
            }
            Ok(())
        }
    }

    /// Saves executed transaction vm output given the `version`.
    pub(crate) fn put_write_set(
        version: Version,
        write_set: &WriteSet,
        batch: &mut impl WriteBatch,
    ) -> Result<()> {
        batch.put::<WriteSetSchema>(&version, write_set)
    }

    /// Deletes the write sets between a range of version in [begin, end).
    pub(crate) fn prune(begin: Version, end: Version, db_batch: &mut SchemaBatch) -> Result<()> {
        for version in begin..end {
            db_batch.delete::<WriteSetSchema>(&version)?;
        }
        Ok(())
    }
}
