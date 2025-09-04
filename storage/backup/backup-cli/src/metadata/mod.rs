// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod cache;
pub mod view;

use crate::storage::{FileHandle, ShellSafeName, TextLine};
use anyhow::{ensure, Result};
use velor_crypto::HashValue;
use velor_types::transaction::Version;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::HashMap, convert::TryInto};

#[derive(Deserialize, Serialize)]
#[allow(clippy::enum_variant_names)] // to introduce: BackupperId, etc
pub(crate) enum Metadata {
    EpochEndingBackup(EpochEndingBackupMeta),
    StateSnapshotBackup(StateSnapshotBackupMeta),
    TransactionBackup(TransactionBackupMeta),
    Identity(IdentityMeta),
    CompactionTimestamps(CompactionTimestampsMeta),
}

impl Metadata {
    pub fn new_epoch_ending_backup(
        first_epoch: u64,
        last_epoch: u64,
        first_version: Version,
        last_version: Version,
        manifest: FileHandle,
    ) -> Self {
        Self::EpochEndingBackup(EpochEndingBackupMeta {
            first_epoch,
            last_epoch,
            first_version,
            last_version,
            manifest,
        })
    }

    pub fn new_state_snapshot_backup(epoch: u64, version: Version, manifest: FileHandle) -> Self {
        Self::StateSnapshotBackup(StateSnapshotBackupMeta {
            epoch,
            version,
            manifest,
        })
    }

    pub fn new_transaction_backup(
        first_version: Version,
        last_version: Version,
        manifest: FileHandle,
    ) -> Self {
        Self::TransactionBackup(TransactionBackupMeta {
            first_version,
            last_version,
            manifest,
        })
    }

    pub fn new_compaction_timestamps(compaction_timestamps_meta: CompactionTimestampsMeta) -> Self {
        Self::CompactionTimestamps(compaction_timestamps_meta)
    }

    pub fn compact_epoch_ending_backup_range(
        backup_metas: Vec<EpochEndingBackupMeta>,
    ) -> Result<(Vec<TextLine>, ShellSafeName)> {
        ensure!(
            !backup_metas.is_empty(),
            "compacting an empty metadata vector"
        );
        let backup_meta = backup_metas[0].clone();
        let first_epoch = backup_meta.first_epoch;
        let mut next_epoch = backup_meta.last_epoch + 1; // non inclusive
        let mut res = Vec::new();
        res.push(Metadata::EpochEndingBackup(backup_meta).to_text_line()?);
        for backup in backup_metas.iter().skip(1) {
            ensure!(
                next_epoch == backup.first_epoch,
                "Epoch ending backup ranges is not continuous expecting epoch {}, got {}",
                next_epoch,
                backup.first_epoch,
            );
            next_epoch = backup.last_epoch + 1;
            res.push(Metadata::EpochEndingBackup(backup.clone()).to_text_line()?)
        }
        let name = format!(
            "epoch_ending_compacted_{}-{}.meta",
            first_epoch,
            next_epoch - 1
        );
        Ok((res, name.parse()?))
    }

    pub fn compact_statesnapshot_backup_range(
        backup_metas: Vec<StateSnapshotBackupMeta>,
    ) -> Result<(Vec<TextLine>, ShellSafeName)> {
        ensure!(
            !backup_metas.is_empty(),
            "compacting an empty metadata vector"
        );
        let name = format!(
            "state_snapshot_compacted_epoch_{}_{}.meta",
            backup_metas[0].epoch,
            backup_metas[backup_metas.len() - 1].epoch
        );
        let res: Vec<TextLine> = backup_metas
            .into_iter()
            .map(|e| Metadata::StateSnapshotBackup(e).to_text_line())
            .collect::<Result<_>>()?;
        Ok((res, name.parse()?))
    }

    pub fn compact_transaction_backup_range(
        backup_metas: Vec<TransactionBackupMeta>,
    ) -> Result<(Vec<TextLine>, ShellSafeName)> {
        ensure!(
            !backup_metas.is_empty(),
            "compacting an empty metadata vector"
        );
        // assume the vector is sorted based on version
        let backup_meta = backup_metas[0].clone();
        let first_version = backup_meta.first_version;
        // assume the last_version is inclusive in the backup meta
        let mut next_version = backup_meta.last_version + 1;
        let mut res: Vec<TextLine> = Vec::new();
        res.push(Metadata::TransactionBackup(backup_meta).to_text_line()?);
        for backup in backup_metas.iter().skip(1) {
            ensure!(
                next_version == backup.first_version,
                "txn backup ranges is not continuous expecting version {}, got {}.",
                next_version,
                backup.first_version,
            );
            next_version = backup.last_version + 1;
            res.push(Metadata::TransactionBackup(backup.clone()).to_text_line()?)
        }
        let name = format!(
            "transaction_compacted_{}-{}.meta",
            first_version,
            next_version - 1
        );
        Ok((res, name.parse()?))
    }

    pub fn new_random_identity() -> Self {
        Self::Identity(IdentityMeta {
            id: HashValue::random(),
        })
    }

    pub fn name(&self) -> ShellSafeName {
        match self {
            Self::EpochEndingBackup(e) => {
                format!("epoch_ending_{}-{}.meta", e.first_epoch, e.last_epoch)
            },
            Self::StateSnapshotBackup(s) => format!("state_snapshot_ver_{}.meta", s.version),
            Self::TransactionBackup(t) => {
                format!("transaction_{}-{}.meta", t.first_version, t.last_version)
            },
            Metadata::Identity(_) => "identity.meta".into(),
            Self::CompactionTimestamps(e) => {
                format!("compaction_timestamps_{}.meta", e.file_compacted_at,)
            },
        }
        .try_into()
        .unwrap()
    }

    pub fn to_text_line(&self) -> Result<TextLine> {
        TextLine::new(&serde_json::to_string(self)?)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct EpochEndingBackupMeta {
    pub first_epoch: u64,
    pub last_epoch: u64,
    pub first_version: Version,
    pub last_version: Version,
    pub manifest: FileHandle,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct StateSnapshotBackupMeta {
    pub epoch: u64,
    pub version: Version,
    pub manifest: FileHandle,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct TransactionBackupMeta {
    pub first_version: Version,
    pub last_version: Version,
    pub manifest: FileHandle,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, Ord, PartialOrd)]
pub struct IdentityMeta {
    pub id: HashValue,
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq)]
pub struct CompactionTimestampsMeta {
    pub file_compacted_at: u64,
    pub compaction_timestamps: HashMap<FileHandle, Option<u64>>,
}

impl CompactionTimestampsMeta {
    pub fn new(
        compaction_timestamps: HashMap<FileHandle, Option<u64>>,
        file_compacted_at: u64,
    ) -> Self {
        Self {
            file_compacted_at,
            compaction_timestamps,
        }
    }
}

impl PartialEq<Self> for CompactionTimestampsMeta {
    fn eq(&self, other: &Self) -> bool {
        self.file_compacted_at.eq(&other.file_compacted_at)
    }
}

impl PartialOrd<Self> for CompactionTimestampsMeta {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompactionTimestampsMeta {
    fn cmp(&self, other: &Self) -> Ordering {
        self.file_compacted_at.cmp(&other.file_compacted_at)
    }
}
