// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Backup / restore helpers for the native-position subsystem: DB-facing
//! reads/writes plus the composite-root cross-check. Archive framing lives
//! in `storage/backup/backup-cli`.

#![forbid(unsafe_code)]

use crate::{
    position_db::PositionDb,
    position_merkle_db::{compose_state_root, PositionMerkleDb},
};
use aptos_crypto::{hash::CryptoHash, HashValue};
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Travels alongside a backup so the receiver can verify against the
/// snapshot's composite state root without reconstructing the JMT.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PositionBackupMetadata {
    pub version: Version,
    pub position_root: HashValue,
}

/// Snapshot every live position at `version`. For large stores prefer
/// [`stream_position_snapshot`] to avoid materializing it all in RAM.
pub fn export_position_snapshot(
    position_db: &Arc<PositionDb>,
    position_merkle_db: &Arc<PositionMerkleDb>,
    version: Version,
) -> Result<Vec<(StateKey, StateValue)>> {
    position_merkle_db
        .iter_active_leaves_with_values(Arc::clone(position_db), version, 0)?
        .collect()
}

/// Streaming export. Currently wraps [`export_position_snapshot`].
pub fn stream_position_snapshot(
    position_db: &Arc<PositionDb>,
    position_merkle_db: &Arc<PositionMerkleDb>,
    version: Version,
) -> Result<impl Iterator<Item = (StateKey, StateValue)>> {
    let rows = export_position_snapshot(position_db, position_merkle_db, version)?;
    Ok(rows.into_iter())
}

/// Load a snapshot into a freshly-initialized `position_db`. Does not
/// populate the in-memory store — pair with
/// `NativeStateStore::populate_from_rows` after restore.
pub fn import_position_snapshot<I>(
    position_db: &Arc<PositionDb>,
    version: Version,
    rows: I,
) -> Result<()>
where
    I: IntoIterator<Item = (StateKey, StateValue)>,
{
    position_db.write_position_batch(version, rows.into_iter().map(|(k, v)| (k.hash(), Some(v))))
}

/// Read `(version, position_root)` to ship alongside the archive.
pub fn snapshot_metadata(
    position_merkle_db: &Arc<PositionMerkleDb>,
    version: Version,
) -> Result<PositionBackupMetadata> {
    Ok(PositionBackupMetadata {
        version,
        position_root: position_merkle_db.get_root_hash(version)?,
    })
}

/// Cross-check backup metadata against the committed composite state root,
/// re-deriving `compose_state_root(main_state_root, position_root)`.
pub fn verify_against_composite_root(
    metadata: &PositionBackupMetadata,
    main_state_root: HashValue,
    expected_composite_root: HashValue,
) -> Result<()> {
    let derived = compose_state_root(main_state_root, metadata.position_root);
    if derived != expected_composite_root {
        return Err(AptosDbError::Other(format!(
            "position backup composite-root mismatch at v={}: derived={derived}, expected={expected_composite_root} \
             (main={main_state_root}, position={})",
            metadata.version, metadata.position_root,
        )));
    }
    Ok(())
}
