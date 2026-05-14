// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Backup / restore helpers for the native-position subsystem.
//!
//! The plan requires `state_kv_db`, `state_merkle_db`, `position_db`,
//! and `position_merkle_db` to be snapshotted at the same `Version`,
//! so backups + composite-root proofs are consistent.
//!
//! Hash-keyed value rows in `position_value` carry only the state-key
//! hash, so the backup pipeline pulls the original [`StateKey`] from
//! the JMT (`position_merkle_db`). Enumerating positions at a
//! snapshot version is therefore a JMT walk that yields
//! [`StateKey`]s; the value is fetched by hash. This module owns the
//! DB-facing reads/writes and the composite-root cross-check; archive
//! framing lives in `storage/backup/backup-cli`.

#![forbid(unsafe_code)]

use crate::{position_db::PositionDb, position_merkle_db::PositionMerkleDb};
use aptos_crypto::hash::CryptoHash;
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
};
use std::sync::Arc;

/// Snapshot every live Position at `version`. Walks the JMT to
/// enumerate `(StateKey, value_hash)` pairs, then queries
/// `position_db` by hash to fetch the value bytes.
///
/// For large stores prefer [`stream_position_snapshot`] to avoid
/// materializing the whole snapshot in RAM.
pub fn export_position_snapshot(
    position_db: &Arc<PositionDb>,
    position_merkle_db: &Arc<PositionMerkleDb>,
    version: Version,
) -> Result<Vec<(StateKey, StateValue)>> {
    let mut out = Vec::new();
    for entry in position_merkle_db.iter_active_leaves(version)? {
        let (state_key, _value_hash) = entry?;
        let key_hash = state_key.hash();
        let value = position_db
            .get_position_value(key_hash, version)?
            .ok_or_else(|| {
                AptosDbError::Other(format!(
                    "export_position_snapshot: JMT leaf at version {version} has no value-CF row \
                 for state_key_hash {key_hash}"
                ))
            })?;
        out.push((state_key, value));
    }
    Ok(out)
}

/// Streaming export. Yields `(StateKey, StateValue)` pairs. Currently
/// wraps [`export_position_snapshot`]; will be replaced by a true
/// JMT-iterator + lookup pipeline once the CF size warrants it.
pub fn stream_position_snapshot(
    position_db: &Arc<PositionDb>,
    position_merkle_db: &Arc<PositionMerkleDb>,
    version: Version,
) -> Result<impl Iterator<Item = (StateKey, StateValue)>> {
    let rows = export_position_snapshot(position_db, position_merkle_db, version)?;
    Ok(rows.into_iter())
}

/// Load a snapshot into a freshly-initialized `position_db`. Writes
/// each value by its `StateKey::hash()` row key at the provided
/// version. The in-memory store is NOT populated by this call —
/// pair with `NativeStateStore::populate_from_rows` after restore.
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
