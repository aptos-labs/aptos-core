// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    schema::{
        block_by_version::BlockByVersionSchema,
        block_info::BlockInfoSchema,
        db_metadata::{DbMetadataKey, DbMetadataSchema, DbMetadataValue},
        epoch_by_version::EpochByVersionSchema,
        ledger_info::LedgerInfoSchema,
        version_data::VersionDataSchema,
    },
    utils::{get_progress, iterators::EpochEndingLedgerInfoIter},
};
use anyhow::anyhow;
use velor_schemadb::{batch::SchemaBatch, DB};
use velor_storage_interface::{block_info::BlockInfo, db_ensure as ensure, VelorDbError, Result};
use velor_types::{
    account_config::NewBlockEvent, block_info::BlockHeight, contract_event::ContractEvent,
    epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
    state_store::state_storage_usage::StateStorageUsage, transaction::Version,
};
use arc_swap::ArcSwap;
use std::{ops::Deref, path::Path, sync::Arc};

fn get_latest_ledger_info_in_db_impl(db: &DB) -> Result<Option<LedgerInfoWithSignatures>> {
    let mut iter = db.iter::<LedgerInfoSchema>()?;
    iter.seek_to_last();
    Ok(iter.next().transpose()?.map(|(_, v)| v))
}

#[derive(Debug)]
pub(crate) struct LedgerMetadataDb {
    db: Arc<DB>,

    /// We almost always need the latest ledger info and signatures to serve read requests, so we
    /// cache it in memory in order to avoid reading DB and deserializing the object frequently. It
    /// should be updated every time new ledger info and signatures are persisted.
    latest_ledger_info: ArcSwap<Option<LedgerInfoWithSignatures>>,
}

impl LedgerMetadataDb {
    pub(super) fn new(db: Arc<DB>) -> Self {
        let latest_ledger_info = get_latest_ledger_info_in_db_impl(&db).expect("DB read failed.");
        let latest_ledger_info = ArcSwap::from(Arc::new(latest_ledger_info));

        Self {
            db,
            latest_ledger_info,
        }
    }

    pub(super) fn create_checkpoint(&self, path: impl AsRef<Path>) -> Result<()> {
        self.db.create_checkpoint(path)
    }

    pub(super) fn write_pruner_progress(&self, version: Version) -> Result<()> {
        self.db.put::<DbMetadataSchema>(
            &DbMetadataKey::LedgerPrunerProgress,
            &DbMetadataValue::Version(version),
        )
    }

    pub(crate) fn db(&self) -> &DB {
        &self.db
    }

    pub(super) fn db_arc(&self) -> Arc<DB> {
        Arc::clone(&self.db)
    }

    pub(crate) fn write_schemas(&self, batch: SchemaBatch) -> Result<()> {
        self.db.write_schemas(batch)
    }

    pub(crate) fn get_synced_version(&self) -> Result<Option<Version>> {
        get_progress(&self.db, &DbMetadataKey::OverallCommitProgress)
    }

    pub(crate) fn get_ledger_commit_progress(&self) -> Result<Version> {
        get_progress(&self.db, &DbMetadataKey::LedgerCommitProgress)?
            .ok_or_else(|| VelorDbError::NotFound("No LedgerCommitProgress in db.".to_string()))
    }

    pub(crate) fn get_pruner_progress(&self) -> Result<Version> {
        get_progress(&self.db, &DbMetadataKey::LedgerPrunerProgress)?
            .ok_or_else(|| VelorDbError::NotFound("No LedgerPrunerProgress in db.".to_string()))
    }
}

/// LedgerInfo APIs.
impl LedgerMetadataDb {
    /// Returns the latest ledger info, or None if it doesn't exist.
    pub(crate) fn get_latest_ledger_info_option(&self) -> Option<LedgerInfoWithSignatures> {
        let ledger_info_ptr = self.latest_ledger_info.load();
        let ledger_info: &Option<_> = ledger_info_ptr.deref();
        ledger_info.clone()
    }

    pub(crate) fn get_committed_version(&self) -> Option<Version> {
        let ledger_info_ptr = self.latest_ledger_info.load();
        let ledger_info: &Option<_> = ledger_info_ptr.deref();
        ledger_info.as_ref().map(|li| li.ledger_info().version())
    }

    /// Returns the latest ledger info, or NOT_FOUND if it doesn't exist.
    pub(crate) fn get_latest_ledger_info(&self) -> Result<LedgerInfoWithSignatures> {
        self.get_latest_ledger_info_option()
            .ok_or_else(|| VelorDbError::NotFound(String::from("Genesis LedgerInfo")))
    }

    /// Returns the latest ledger info for a given epoch.
    pub(crate) fn get_latest_ledger_info_in_epoch(
        &self,
        epoch: u64,
    ) -> Result<LedgerInfoWithSignatures> {
        self.db
            .get::<LedgerInfoSchema>(&epoch)?
            .ok_or_else(|| VelorDbError::NotFound(format!("Last LedgerInfo of epoch {epoch}")))
    }

    /// Returns an iterator that yields epoch ending ledger infos, starting from `start_epoch`, and
    /// ends at the one before `end_epoch`.
    pub(crate) fn get_epoch_ending_ledger_info_iter(
        &self,
        start_epoch: u64,
        end_epoch: u64,
    ) -> Result<EpochEndingLedgerInfoIter> {
        let mut iter = self.db.iter::<LedgerInfoSchema>()?;
        iter.seek(&start_epoch)?;
        Ok(EpochEndingLedgerInfoIter::new(iter, start_epoch, end_epoch))
    }

    /// Returns the epoch state for the given epoch.
    pub(crate) fn get_epoch_state(&self, epoch: u64) -> Result<EpochState> {
        ensure!(epoch > 0, "EpochState only queryable for epoch >= 1.",);

        let ledger_info_with_sigs =
            self.db
                .get::<LedgerInfoSchema>(&(epoch - 1))?
                .ok_or_else(|| {
                    VelorDbError::NotFound(format!("Last LedgerInfo of epoch {}", epoch - 1))
                })?;
        let latest_epoch_state = ledger_info_with_sigs
            .ledger_info()
            .next_epoch_state()
            .ok_or_else(|| {
                VelorDbError::Other(
                    "Last LedgerInfo in epoch must carry next_epoch_state.".to_string(),
                )
            })?;

        Ok(latest_epoch_state.clone())
    }

    /// Returns ledger info at a specified version, and ensures it's an epoch ending.
    pub(crate) fn get_epoch_ending_ledger_info(
        &self,
        version: Version,
    ) -> Result<LedgerInfoWithSignatures> {
        let epoch = self.get_epoch(version)?;
        let li = self
            .db
            .get::<LedgerInfoSchema>(&epoch)?
            .ok_or_else(|| VelorDbError::NotFound(format!("LedgerInfo for epoch {}.", epoch)))?;
        ensure!(
            li.ledger_info().version() == version,
            "Epoch {} didn't end at version {}",
            epoch,
            version,
        );
        li.ledger_info().next_epoch_state().ok_or_else(|| {
            VelorDbError::NotFound(format!("Not an epoch change at version {version}"))
        })?;

        Ok(li)
    }

    /// Stores the latest ledger info in memory.
    pub(crate) fn set_latest_ledger_info(&self, ledger_info_with_sigs: LedgerInfoWithSignatures) {
        self.latest_ledger_info
            .store(Arc::new(Some(ledger_info_with_sigs)));
    }

    /// Writes `ledger_info_with_sigs` to `batch`.
    pub(crate) fn put_ledger_info(
        &self,
        ledger_info_with_sigs: &LedgerInfoWithSignatures,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        let ledger_info = ledger_info_with_sigs.ledger_info();

        if ledger_info.ends_epoch() {
            // This is the last version of the current epoch, update the epoch by version index.
            batch.put::<EpochByVersionSchema>(&ledger_info.version(), &ledger_info.epoch())?;
        }
        batch.put::<LedgerInfoSchema>(&ledger_info.epoch(), ledger_info_with_sigs)
    }
}

/// Epoch APIs.
impl LedgerMetadataDb {
    /// Returns the epoch at the given version.
    pub(crate) fn get_epoch(&self, version: Version) -> Result<u64> {
        let mut iter = self.db.iter::<EpochByVersionSchema>()?;
        // Search for the end of the previous epoch.
        iter.seek_for_prev(&version)?;
        let (epoch_end_version, epoch) = match iter.next().transpose()? {
            Some(x) => x,
            None => {
                // There should be a genesis LedgerInfo at version 0 (genesis only consists of one
                // transaction), so this normally doesn't happen. However this part of
                // implementation doesn't need to rely on this assumption.
                return Ok(0);
            },
        };
        ensure!(
            epoch_end_version <= version,
            "DB corruption: looking for epoch for version {}, got epoch {} ends at version {}",
            version,
            epoch,
            epoch_end_version
        );
        // If the obtained epoch ended before the given version, return epoch+1, otherwise
        // the given version is exactly the last version of the found epoch.
        Ok(if epoch_end_version < version {
            epoch + 1
        } else {
            epoch
        })
    }

    /// Returns error if the given version is not epoch ending version.
    pub(crate) fn ensure_epoch_ending(&self, version: Version) -> Result<()> {
        self.db
            .get::<EpochByVersionSchema>(&version)?
            .ok_or_else(|| {
                VelorDbError::Other(format!("Version {version} is not epoch ending."))
            })?;

        Ok(())
    }

    /// Returns the latest ended epoch strictly before required version, i.e. if the passed in
    /// version ends an epoch, return one epoch early than that.
    pub(crate) fn get_previous_epoch_ending(
        &self,
        version: Version,
    ) -> Result<Option<(u64, Version)>> {
        if version == 0 {
            return Ok(None);
        }
        let prev_version = version - 1;

        let mut iter = self.db.iter::<EpochByVersionSchema>()?;
        // Search for the end of the previous epoch.
        iter.seek_for_prev(&prev_version)?;
        iter.next().transpose()
    }
}

/// Block APIs.
impl LedgerMetadataDb {
    /// Returns the BlockInfo for the given block_height, or None if it doesn't exist in database.
    pub(crate) fn get_block_info(&self, block_height: u64) -> Result<Option<BlockInfo>> {
        self.db.get::<BlockInfoSchema>(&block_height)
    }

    /// Returns the corresponding block height for a given version.
    pub(crate) fn get_block_height_by_version(&self, version: Version) -> Result<u64> {
        let mut iter = self.db.iter::<BlockByVersionSchema>()?;

        iter.seek_for_prev(&version)?;
        let (_, block_height) = iter
            .next()
            .transpose()?
            .ok_or_else(|| anyhow!("Block is not found at version {version}, maybe pruned?"))?;

        Ok(block_height)
    }

    pub(crate) fn get_block_height_at_or_after_version(
        &self,
        version: Version,
    ) -> Result<(Version, BlockHeight)> {
        let mut iter = self.db.iter::<BlockByVersionSchema>()?;
        iter.seek(&version)?;
        let (block_version, block_height) = iter
            .next()
            .transpose()?
            .ok_or_else(|| anyhow!("Block is not found at or after version {version}"))?;

        Ok((block_version, block_height))
    }

    pub(crate) fn put_block_info(
        version: Version,
        event: &ContractEvent,
        batch: &mut SchemaBatch,
    ) -> Result<()> {
        let new_block_event = NewBlockEvent::try_from_bytes(event.event_data())?;
        let block_height = new_block_event.height();
        let block_info = BlockInfo::from_new_block_event(version, &new_block_event);
        batch.put::<BlockInfoSchema>(&block_height, &block_info)?;
        batch.put::<BlockByVersionSchema>(&version, &block_height)?;

        Ok(())
    }
}

/// Usage APIs.
impl LedgerMetadataDb {
    /// Returns the state usage, or error if it doesn't exist in database.
    pub(crate) fn get_usage(&self, version: Version) -> Result<StateStorageUsage> {
        Ok(self
            .db
            .get::<VersionDataSchema>(&version)?
            .ok_or_else(|| anyhow!("VersionData missing for version {version}"))?
            .get_state_storage_usage())
    }

    /// Writes the state usage to database.
    pub(crate) fn put_usage(&self, version: Version, usage: StateStorageUsage) -> Result<()> {
        self.db.put::<VersionDataSchema>(&version, &usage.into())
    }

    pub(crate) fn get_usage_before_or_at(
        &self,
        version: Version,
    ) -> Result<(Version, StateStorageUsage)> {
        let mut iter = self.db.iter::<VersionDataSchema>()?;
        iter.seek_for_prev(&version)?;
        match iter.next().transpose()? {
            Some((previous_version, data)) => {
                Ok((previous_version, data.get_state_storage_usage()))
            },
            None => Err(VelorDbError::NotFound(
                "Unable to find a version before the given version with usage.".to_string(),
            )),
        }
    }
}
