// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    error::Error,
    metadata_storage::database_schema::{MetadataKey, MetadataSchema, MetadataValue},
};
use anyhow::{anyhow, Result};
use aptos_logger::prelude::*;
use aptos_schemadb::{
    batch::SchemaBatch,
    define_schema,
    schema::{KeyCodec, ValueCodec},
    ColumnFamilyName, Options, DB,
};
use aptos_storage_interface::StateKind;
use aptos_types::ledger_info::LedgerInfoWithSignatures;
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc, time::Instant};

/// The metadata storage interface required by state sync. This enables
/// state sync to handle failures and reboots during critical parts
/// of the syncing process, where a failure may cause an inconsistent
/// state to remain in the database on startup.
pub trait MetadataStorageInterface {
    /// Returns true iff a snapshot of the given `kind` was successfully
    /// committed for the specified target.
    /// If no snapshot progress is found, an error is returned.
    fn is_snapshot_sync_complete(
        &self,
        target_ledger_info: &LedgerInfoWithSignatures,
        kind: StateKind,
    ) -> Result<bool, Error>;

    /// Gets the last persisted value index for the `kind` snapshot sync at the
    /// specified version. If no snapshot progress is found, an error is returned.
    fn get_last_persisted_index(
        &self,
        target_ledger_info: &LedgerInfoWithSignatures,
        kind: StateKind,
    ) -> Result<u64, Error>;

    /// Returns the target ledger info of any `kind` snapshot sync that has
    /// previously started. If no snapshot sync started, None is returned.
    fn previous_snapshot_sync_target(
        &self,
        kind: StateKind,
    ) -> Result<Option<LedgerInfoWithSignatures>, Error>;

    /// Updates the last persisted value index for the `kind` snapshot sync at
    /// the specified target ledger info.
    fn update_last_persisted_index(
        &self,
        target_ledger_info: &LedgerInfoWithSignatures,
        last_persisted_index: u64,
        snapshot_sync_completed: bool,
        kind: StateKind,
    ) -> Result<(), Error>;
}

/// The `MetadataKey` for the given snapshot kind's progress row. Each kind is an
/// independent row carrying a `StateSnapshotProgress`.
fn snapshot_metadata_key(kind: StateKind) -> MetadataKey {
    match kind {
        StateKind::MainState => MetadataKey::StateSnapshotSync,
        StateKind::Position => MetadataKey::PositionSnapshotSync,
    }
}

/// The `MetadataValue` wrapping `progress` for the given snapshot kind.
fn snapshot_metadata_value(kind: StateKind, progress: StateSnapshotProgress) -> MetadataValue {
    match kind {
        StateKind::MainState => MetadataValue::StateSnapshotSync(progress),
        StateKind::Position => MetadataValue::PositionSnapshotSync(progress),
    }
}

/// A short label for the given snapshot kind, used in log/error messages.
fn snapshot_kind_label(kind: StateKind) -> &'static str {
    match kind {
        StateKind::MainState => "state",
        StateKind::Position => "position",
    }
}

/// The name of the state sync db file
pub const STATE_SYNC_DB_NAME: &str = "state_sync_db";

/// The name of the metadata column family
const METADATA_CF_NAME: ColumnFamilyName = "metadata";

/// A metadata storage implementation that uses a RocksDB backend to persist data
#[derive(Clone)]
pub struct PersistentMetadataStorage {
    database: Arc<DB>,
}

impl PersistentMetadataStorage {
    pub fn new<P: AsRef<Path> + Clone>(db_root_path: P) -> Self {
        // Set the options to create the database if it's missing
        let mut options = Options::default();
        options.create_if_missing(true);
        options.create_missing_column_families(true);

        // Open the database
        let state_sync_db_path = db_root_path.as_ref().join(STATE_SYNC_DB_NAME);
        let instant = Instant::now();
        let database = DB::open(
            state_sync_db_path.clone(),
            "state_sync",
            vec![METADATA_CF_NAME],
            options,
        )
        .unwrap_or_else(|error| {
            panic!(
                "Failed to open/create the state sync database at: {:?}. Error: {:?}",
                state_sync_db_path, error
            )
        });
        info!(
            "Opened the state sync database at: {:?}, in {:?} ms",
            state_sync_db_path,
            instant.elapsed().as_millis()
        );

        let database = Arc::new(database);
        Self { database }
    }

    /// Returns the existing snapshot sync progress for `kind`. None if not found.
    fn get_snapshot_progress(
        &self,
        kind: StateKind,
    ) -> Result<Option<StateSnapshotProgress>, Error> {
        let metadata_key = snapshot_metadata_key(kind);
        let maybe_metadata_value =
            self.database
                .get::<MetadataSchema>(&metadata_key)
                .map_err(|error| {
                    Error::StorageError(format!(
                        "Failed to read metadata value for key: {:?}. Error: {:?}",
                        metadata_key, error
                    ))
                })?;
        // Each kind is stored under its own key, so a value of the other variant
        // is never expected; treat it (and a missing row) as no progress.
        let progress = match maybe_metadata_value {
            None => None,
            Some(MetadataValue::StateSnapshotSync(progress)) => match kind {
                StateKind::MainState => Some(progress),
                StateKind::Position => None,
            },
            Some(MetadataValue::PositionSnapshotSync(progress)) => match kind {
                StateKind::Position => Some(progress),
                StateKind::MainState => None,
            },
        };
        Ok(progress)
    }

    /// Returns the snapshot sync progress recorded for `kind` at the specified
    /// target. Returns an error if no progress was found.
    fn get_snapshot_progress_at_target(
        &self,
        kind: StateKind,
        target_ledger_info: &LedgerInfoWithSignatures,
    ) -> Result<StateSnapshotProgress, Error> {
        match self.get_snapshot_progress(kind)? {
            Some(snapshot_progress) => {
                if &snapshot_progress.target_ledger_info != target_ledger_info {
                    Err(Error::UnexpectedError(format!(
                        "Expected a {} snapshot progress for target {:?}, but found {:?}!",
                        snapshot_kind_label(kind),
                        target_ledger_info,
                        snapshot_progress.target_ledger_info
                    )))
                } else {
                    Ok(snapshot_progress)
                }
            },
            None => Err(Error::StorageError(format!(
                "No {} snapshot progress was found!",
                snapshot_kind_label(kind)
            ))),
        }
    }

    /// Write the key value pair to the database
    fn commit_key_value(
        &self,
        metadata_key: MetadataKey,
        metadata_value: MetadataValue,
    ) -> Result<(), Error> {
        // Create the schema batch
        let mut batch = SchemaBatch::new();
        batch
            .put::<MetadataSchema>(&metadata_key, &metadata_value)
            .map_err(|error| {
                Error::StorageError(format!(
                    "Failed to batch put the metadata key and value. Key: {:?}, Value: {:?}. Error: {:?}", metadata_key, metadata_value, error
                ))
            })?;

        // Write the schema batch to the database
        self.database.write_schemas(batch).map_err(|error| {
            Error::StorageError(format!(
                "Failed to write the metadata schema. Error: {:?}",
                error
            ))
        })
    }

    /// Creates new physical DB checkpoint in directory specified by `path`.
    pub fn create_checkpoint<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let start = Instant::now();
        let state_sync_db_path = path.as_ref().join(STATE_SYNC_DB_NAME);
        std::fs::remove_dir_all(&state_sync_db_path).unwrap_or(());
        self.database.create_checkpoint(&state_sync_db_path)?;
        info!(
            path = state_sync_db_path,
            time_ms = %start.elapsed().as_millis(),
            "Made StateSyncDB checkpoint."
        );
        Ok(())
    }
}

impl MetadataStorageInterface for PersistentMetadataStorage {
    fn is_snapshot_sync_complete(
        &self,
        target: &LedgerInfoWithSignatures,
        kind: StateKind,
    ) -> Result<bool, Error> {
        let snapshot_progress = self.get_snapshot_progress_at_target(kind, target)?;
        Ok(snapshot_progress.snapshot_sync_completed)
    }

    fn get_last_persisted_index(
        &self,
        target: &LedgerInfoWithSignatures,
        kind: StateKind,
    ) -> Result<u64, Error> {
        let snapshot_progress = self.get_snapshot_progress_at_target(kind, target)?;
        Ok(snapshot_progress.last_persisted_state_value_index)
    }

    fn previous_snapshot_sync_target(
        &self,
        kind: StateKind,
    ) -> Result<Option<LedgerInfoWithSignatures>, Error> {
        Ok(self
            .get_snapshot_progress(kind)?
            .map(|snapshot_progress| snapshot_progress.target_ledger_info))
    }

    fn update_last_persisted_index(
        &self,
        target_ledger_info: &LedgerInfoWithSignatures,
        last_persisted_index: u64,
        snapshot_sync_completed: bool,
        kind: StateKind,
    ) -> Result<(), Error> {
        // Ensure any existing progress for this kind is for the same target.
        if let Some(snapshot_progress) = self.get_snapshot_progress(kind)? {
            if target_ledger_info != &snapshot_progress.target_ledger_info {
                return Err(Error::StorageError(format!("Failed to update the last persisted {} index! \
                The given target does not match the previously stored target. Given target: {:?}, stored target: {:?}",
                    snapshot_kind_label(kind), target_ledger_info, snapshot_progress.target_ledger_info
                )));
            }
        }

        let metadata_value = snapshot_metadata_value(kind, StateSnapshotProgress {
            last_persisted_state_value_index: last_persisted_index,
            snapshot_sync_completed,
            target_ledger_info: target_ledger_info.clone(),
        });
        self.commit_key_value(snapshot_metadata_key(kind), metadata_value)
    }
}

/// A simple struct for recording the progress of a state snapshot sync
#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct StateSnapshotProgress {
    pub target_ledger_info: LedgerInfoWithSignatures,
    pub last_persisted_state_value_index: u64,
    pub snapshot_sync_completed: bool,
}

/// The raw schema format used by the database
pub mod database_schema {
    use super::*;

    // This defines a physical storage schema for any metadata.
    //
    // The key will be a bcs serialized MetadataKey type.
    // The value will be a bcs serialized MetadataValue type.
    //
    // |<-------key------->|<-----value----->|
    // |   metadata key    | metadata value  |
    define_schema!(MetadataSchema, MetadataKey, MetadataValue, METADATA_CF_NAME);

    /// A metadata key that can be inserted into the database
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[repr(u8)]
    pub enum MetadataKey {
        StateSnapshotSync,    // A state snapshot sync that was started
        PositionSnapshotSync, // A native-position snapshot sync that was started
    }

    /// A metadata value that can be inserted into the database
    #[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
    #[repr(u8)]
    pub enum MetadataValue {
        StateSnapshotSync(StateSnapshotProgress), // A state snapshot sync progress marker
        PositionSnapshotSync(StateSnapshotProgress), // A native-position snapshot sync progress marker
    }

    impl KeyCodec<MetadataSchema> for MetadataKey {
        fn encode_key(&self) -> Result<Vec<u8>> {
            bcs::to_bytes(self).map_err(|error| {
                anyhow!(
                    "Failed to encode metadata key: {:?}. Error: {:?}",
                    self,
                    error
                )
            })
        }

        fn decode_key(data: &[u8]) -> Result<Self> {
            bcs::from_bytes::<MetadataKey>(data).map_err(|error| {
                anyhow!(
                    "Failed to decode metadata key: {:?}. Error: {:?}",
                    data,
                    error
                )
            })
        }
    }

    impl ValueCodec<MetadataSchema> for MetadataValue {
        fn encode_value(&self) -> Result<Vec<u8>> {
            bcs::to_bytes(self).map_err(|error| {
                anyhow!(
                    "Failed to encode metadata value: {:?}. Error: {:?}",
                    self,
                    error
                )
            })
        }

        fn decode_value(data: &[u8]) -> Result<Self> {
            bcs::from_bytes::<MetadataValue>(data).map_err(|error| {
                anyhow!(
                    "Failed to decode metadata value: {:?}. Error: {:?}",
                    data,
                    error
                )
            })
        }
    }
}
