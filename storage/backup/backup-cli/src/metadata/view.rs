// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    metadata::{
        CompactionTimestampsMeta, EpochEndingBackupMeta, IdentityMeta, Metadata,
        StateSnapshotBackupMeta, TransactionBackupMeta,
    },
    metrics::backup::COMPACTED_TXN_VERSION,
    storage::FileHandle,
};
use anyhow::{anyhow, ensure, Result};
use velor_infallible::duration_since_epoch;
use velor_types::transaction::Version;
use itertools::Itertools;
use std::{fmt, str::FromStr};

#[derive(Debug)]
pub struct MetadataView {
    epoch_ending_backups: Vec<EpochEndingBackupMeta>,
    state_snapshot_backups: Vec<StateSnapshotBackupMeta>,
    transaction_backups: Vec<TransactionBackupMeta>,
    _identity: Option<IdentityMeta>,
    // The compaction timestamps of the file handles producing this view
    compaction_timestamps: Option<CompactionTimestampsMeta>,
}

impl MetadataView {
    pub(crate) fn new(metadata_vec: Vec<Metadata>, file_handles: Vec<FileHandle>) -> Self {
        let mut epoch_ending_backups = Vec::new();
        let mut state_snapshot_backups = Vec::new();
        let mut transaction_backups = Vec::new();
        let mut identity = None;
        let mut compaction_timestamps = Vec::new();

        for meta in metadata_vec {
            match meta {
                Metadata::EpochEndingBackup(e) => epoch_ending_backups.push(e),
                Metadata::StateSnapshotBackup(s) => state_snapshot_backups.push(s),
                Metadata::TransactionBackup(t) => transaction_backups.push(t),
                Metadata::Identity(i) => identity = Some(i),
                Metadata::CompactionTimestamps(t) => compaction_timestamps.push(t),
            }
        }
        epoch_ending_backups.sort_unstable();
        epoch_ending_backups.dedup();
        state_snapshot_backups.sort_unstable();
        state_snapshot_backups.dedup();
        transaction_backups.sort_unstable();
        transaction_backups.dedup();

        let mut compaction_meta_opt = compaction_timestamps.iter().max().cloned();
        if let Some(ref mut compaction_meta) = compaction_meta_opt {
            // insert new_files into the previous_compaction_timestamps
            for file in file_handles.into_iter() {
                // if file is not in timestamps, set it to None, otherwise, keep it the same
                compaction_meta
                    .compaction_timestamps
                    .entry(file)
                    .or_insert(None);
            }
        } else {
            // Create new compaction timestamp meta with new files only
            let compaction_timestamps = file_handles.into_iter().map(|file| (file, None)).collect();
            compaction_meta_opt = Some(CompactionTimestampsMeta {
                file_compacted_at: duration_since_epoch().as_secs(),
                compaction_timestamps,
            });
        };

        Self {
            epoch_ending_backups,
            state_snapshot_backups,
            transaction_backups,
            _identity: identity,
            compaction_timestamps: compaction_meta_opt,
        }
    }

    pub fn get_storage_state(&self) -> Result<BackupStorageState> {
        let latest_epoch_ending_epoch =
            self.epoch_ending_backups.iter().map(|e| e.last_epoch).max();
        let latest_state_snapshot = self.select_state_snapshot(Version::MAX)?;
        let (latest_state_snapshot_epoch, latest_state_snapshot_version) =
            match latest_state_snapshot {
                Some(snapshot) => (Some(snapshot.epoch), Some(snapshot.version)),
                None => (None, None),
            };
        let latest_transaction_version = self
            .transaction_backups
            .iter()
            .map(|t| t.last_version)
            .max();

        Ok(BackupStorageState {
            latest_epoch_ending_epoch,
            latest_state_snapshot_epoch,
            latest_state_snapshot_version,
            latest_transaction_version,
        })
    }

    pub fn select_latest_compaction_timestamps(&self) -> Option<CompactionTimestampsMeta> {
        self.compaction_timestamps.clone()
    }

    pub fn all_state_snapshots(&self) -> &[StateSnapshotBackupMeta] {
        &self.state_snapshot_backups
    }

    pub fn select_state_snapshot(
        &self,
        target_version: Version,
    ) -> Result<Option<StateSnapshotBackupMeta>> {
        Ok(self
            .state_snapshot_backups
            .iter()
            .sorted()
            .rev()
            .find(|m| m.version <= target_version)
            .cloned())
    }

    pub fn expect_state_snapshot(&self, version: Version) -> Result<StateSnapshotBackupMeta> {
        self.state_snapshot_backups
            .iter()
            .find(|m| m.version == version)
            .cloned()
            .ok_or_else(|| anyhow!("State snapshot not found at version {}", version))
    }

    pub fn select_transaction_backups(
        &self,
        start_version: Version,
        target_version: Version,
    ) -> Result<Vec<TransactionBackupMeta>> {
        // This can be more flexible, but for now we assume and check backups are continuous in
        // range (which is always true when we backup from a single backup coordinator)
        let mut next_ver = 0;
        let mut res = Vec::new();
        for backup in self.transaction_backups.iter().sorted() {
            if backup.first_version > target_version {
                break;
            }
            ensure!(
                backup.first_version == next_ver,
                "Transaction backup ranges not continuous, expecting version {}, got {}.",
                next_ver,
                backup.first_version,
            );

            if backup.last_version >= start_version {
                res.push(backup.clone());
            }

            next_ver = backup.last_version + 1;
        }

        Ok(res)
    }

    pub fn max_transaction_version(&self) -> Result<Option<Version>> {
        Ok(self
            .transaction_backups
            .iter()
            .sorted()
            .next_back()
            .map(|backup| backup.last_version))
    }

    pub fn select_epoch_ending_backups(
        &self,
        target_version: Version,
    ) -> Result<Vec<EpochEndingBackupMeta>> {
        // This can be more flexible, but for now we assume and check backups are continuous in
        // range (which is always true when we backup from a single backup coordinator)
        let mut next_epoch = 0;
        let mut res = Vec::new();
        for backup in self.epoch_ending_backups.iter().sorted() {
            if backup.first_version > target_version {
                break;
            }

            ensure!(
                backup.first_epoch == next_epoch,
                "Epoch ending backup ranges not continuous, expecting epoch {}, got {}.",
                next_epoch,
                backup.first_epoch,
            );
            res.push(backup.clone());

            next_epoch = backup.last_epoch + 1;
        }

        Ok(res)
    }

    /// Compact the epoch ending metdata files and merge compaction_cnt files into 1 metadata file
    /// The generated chunks should be sorted based on version
    pub fn compact_backups<T>(backups: &[T], compaction_cnt: usize) -> Result<Vec<&[T]>> {
        // Initialize an empty vector to store the output
        let mut output_vec = Vec::new();

        // Iterate over the input vector in chunks of compaction_cnt
        for chunk in backups.chunks(compaction_cnt) {
            // Create a new vector containing the current chunk
            let new_slice = chunk;
            // Add the new vector to the output vector
            output_vec.push(new_slice);
        }
        // Return the output vector
        Ok(output_vec)
    }

    pub fn compact_epoch_ending_backups(
        &mut self,
        compaction_cnt: usize,
    ) -> Result<Vec<&[EpochEndingBackupMeta]>> {
        Self::compact_backups(&self.epoch_ending_backups, compaction_cnt)
    }

    pub fn compact_transaction_backups(
        &mut self,
        compaction_cnt: usize,
    ) -> Result<Vec<&[TransactionBackupMeta]>> {
        COMPACTED_TXN_VERSION.set(
            self.transaction_backups
                .last()
                .map_or(0, |e| e.first_version) as i64,
        );
        Self::compact_backups(&self.transaction_backups, compaction_cnt)
    }

    pub fn compact_state_backups(
        &mut self,
        compaction_cnt: usize,
    ) -> Result<Vec<&[StateSnapshotBackupMeta]>> {
        Self::compact_backups(&self.state_snapshot_backups, compaction_cnt)
    }

    pub fn get_file_handles(&self) -> Vec<FileHandle> {
        self.select_latest_compaction_timestamps()
            .as_ref()
            .map(|t| t.compaction_timestamps.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default()
    }
}

pub struct BackupStorageState {
    pub latest_epoch_ending_epoch: Option<u64>,
    pub latest_state_snapshot_epoch: Option<u64>,
    pub latest_state_snapshot_version: Option<Version>,
    pub latest_transaction_version: Option<Version>,
}

impl fmt::Display for BackupStorageState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "latest_epoch_ending_epoch: {}, latest_state_snapshot_epoch: {}, latest_state_snapshot_version: {}, latest_transaction_version: {}",
            self.latest_epoch_ending_epoch.as_ref().map_or_else(|| "none".to_string(), u64::to_string),
            self.latest_state_snapshot_epoch.as_ref().map_or_else(|| "none".to_string(), u64::to_string),
            self.latest_state_snapshot_version.as_ref().map_or_else(|| "none".to_string(), Version::to_string),
            self.latest_transaction_version.as_ref().map_or_else(|| "none".to_string(), Version::to_string),
        )
    }
}

trait ParseOptionU64 {
    fn parse_option_u64(&self) -> Result<Option<u64>>;
}

impl ParseOptionU64 for Option<regex::Match<'_>> {
    fn parse_option_u64(&self) -> Result<Option<u64>> {
        let m = self.ok_or_else(|| anyhow!("No match."))?;
        if m.as_str() == "none" {
            Ok(None)
        } else {
            Ok(Some(m.as_str().parse()?))
        }
    }
}

impl FromStr for BackupStorageState {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let captures = regex::Regex::new(
            r"latest_epoch_ending_epoch: (none|\d+), latest_state_snapshot_epoch: (none|\d+), latest_state_snapshot_version: (none|\d+), latest_transaction_version: (none|\d+)",
        )?.captures(s).ok_or_else(|| anyhow!("Not in BackupStorageState display format: {}", s))?;

        Ok(Self {
            latest_epoch_ending_epoch: captures.get(1).parse_option_u64()?,
            latest_state_snapshot_epoch: captures.get(2).parse_option_u64()?,
            latest_state_snapshot_version: captures.get(3).parse_option_u64()?,
            latest_transaction_version: captures.get(4).parse_option_u64()?,
        })
    }
}
