// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::OTHER_TIMERS_SECONDS,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        write_set::WriteSetSchema,
    },
    utils::iterators::ExpectContinuousVersions,
};
use aptos_experimental_runtimes::thread_manager::optimal_min_len;
use aptos_metrics_core::TimerHelper;
use aptos_schemadb::{SchemaBatch, DB};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use aptos_types::{transaction::Version, write_set::WriteSet};
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
            .ok_or_else(|| AptosDbError::NotFound(format!("WriteSet at version {}", version)))
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
                AptosDbError::NotFound(format!("Write set missing for version {}", current_version))
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
    pub(crate) fn commit_write_sets<'a>(
        &self,
        first_version: Version,
        write_sets: impl IndexedParallelIterator<Item = &'a WriteSet>,
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_write_sets"]);

        let batch = SchemaBatch::new();
        let num_txns = write_sets.len();

        write_sets
            .with_min_len(optimal_min_len(num_txns, 128))
            .enumerate()
            .try_for_each(|(i, write_set)| -> Result<()> {
                Self::put_write_set(first_version + i as Version, write_set, &batch)?;

                Ok(())
            })?;
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_write_sets___commit"]);
        self.write_schemas(batch)
    }

    /// Saves executed transaction vm output given the `version`.
    pub(crate) fn put_write_set(
        version: Version,
        write_set: &WriteSet,
        batch: &SchemaBatch,
    ) -> Result<()> {
        batch.put::<WriteSetSchema>(&version, write_set)
    }

    /// Deletes the write sets between a range of version in [begin, end).
    pub(crate) fn prune(begin: Version, end: Version, db_batch: &SchemaBatch) -> Result<()> {
        for version in begin..end {
            db_batch.delete::<WriteSetSchema>(&version)?;
        }
        Ok(())
    }
}
