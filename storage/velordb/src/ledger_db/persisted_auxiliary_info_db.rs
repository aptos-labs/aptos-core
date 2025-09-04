// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metrics::OTHER_TIMERS_SECONDS,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        persisted_auxiliary_info::PersistedAuxiliaryInfoSchema,
    },
    utils::iterators::ExpectContinuousVersions,
};
use velor_metrics_core::TimerHelper;
use velor_schemadb::{batch::SchemaBatch, DB};
use velor_storage_interface::Result;
use velor_types::transaction::{PersistedAuxiliaryInfo, Version};
use std::{path::Path, sync::Arc};

#[derive(Debug)]
pub(crate) struct PersistedAuxiliaryInfoDb {
    db: Arc<DB>,
}

impl PersistedAuxiliaryInfoDb {
    pub(super) fn new(db: Arc<DB>) -> Self {
        Self { db }
    }

    pub(super) fn create_checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        self.db.create_checkpoint(path)
    }

    pub(super) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.db.put::<DbMetadataSchema>(
            &DbMetadataKey::PersistedAuxiliaryInfoPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(super) fn db(&self) -> &DB {
        &self.db
    }

    pub(crate) fn write_schemas(&self, batch: SchemaBatch) -> Result<()> {
        self.db.write_schemas(batch)
    }

    pub(crate) fn get_persisted_auxiliary_info(
        &self,
        version: Version,
    ) -> Result<Option<PersistedAuxiliaryInfo>> {
        self.db.get::<PersistedAuxiliaryInfoSchema>(&version)
    }

    /// Returns an iterator that yields `num_persisted_auxiliary_info` persisted_auxiliary_info
    /// starting from `start_version`.
    ///
    /// Requires the caller to not query the data beyond the latest version.
    pub(crate) fn get_persisted_auxiliary_info_iter(
        &self,
        start_version: Version,
        num_persisted_auxiliary_info: usize,
    ) -> Result<Box<dyn Iterator<Item = Result<PersistedAuxiliaryInfo>> + '_>> {
        let mut iter = self.db.iter::<PersistedAuxiliaryInfoSchema>()?;
        iter.seek(&start_version)?;
        let mut iter = iter.peekable();
        let item = iter.peek();
        let version = if item.is_some() {
            item.unwrap().as_ref().map_err(|e| e.clone())?.0
        } else {
            let mut iter = self.db.iter::<PersistedAuxiliaryInfoSchema>()?;
            iter.seek_to_last();
            if iter.next().transpose()?.is_some() {
                return Ok(Box::new(std::iter::empty()));
            }
            // Note in this case we return all Nones. We rely on the caller to not query future
            // data when the DB is empty.
            // TODO(grao): This will be unreachable in the future, consider make it an error later.
            start_version + num_persisted_auxiliary_info as u64
        };
        let num_none = std::cmp::min(
            num_persisted_auxiliary_info,
            version.saturating_sub(start_version) as usize,
        );
        let none_iter = itertools::repeat_n(Ok(PersistedAuxiliaryInfo::None), num_none);
        Ok(Box::new(none_iter.chain(iter.expect_continuous_versions(
            start_version + num_none as u64,
            num_persisted_auxiliary_info - num_none,
        )?)))
    }

    pub(crate) fn commit_auxiliary_info(
        &self,
        first_version: Version,
        persisted_auxiliary_info: &[PersistedAuxiliaryInfo],
    ) -> Result<()> {
        let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_auxiliary_info"]);

        let mut batch = SchemaBatch::new();
        persisted_auxiliary_info.iter().enumerate().try_for_each(
            |(i, aux_info)| -> Result<()> {
                let version = first_version + i as u64;
                Self::put_persisted_auxiliary_info(version, aux_info, &mut batch)
            },
        )?;

        {
            let _timer = OTHER_TIMERS_SECONDS.timer_with(&["commit_auxiliary_info___commit"]);
            self.write_schemas(batch)
        }
    }

    pub(crate) fn put_persisted_auxiliary_info(
        version: Version,
        persisted_info: &PersistedAuxiliaryInfo,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        batch.put::<PersistedAuxiliaryInfoSchema>(&version, persisted_info)
    }

    /// Deletes the persisted auxiliary info between a range of version in [begin, end)
    pub(crate) fn prune(begin: Version, end: Version, batch: &mut SchemaBatch) -> Result<()> {
        for version in begin..end {
            batch.delete::<PersistedAuxiliaryInfoSchema>(&version)?;
        }
        Ok(())
    }
}
