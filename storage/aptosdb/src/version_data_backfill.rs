// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Backfill (or verify) `VersionData` entries at epoch-ending versions.
//!
//! Each entry of `EpochByVersionSchema` is an epoch-ending version `N`. At
//! `N + 1` the first `BlockMetadata` of the new epoch runs `block_prologue`,
//! which calls `aptos_framework::state_storage::on_new_block`. On an epoch
//! change, that function overwrites the on-chain `StateStorageUsage` resource
//! with the storage usage observed in the block's base state view — i.e. the
//! cumulative `(items, bytes)` at the end of version `N`. The write set at
//! `N + 1` therefore carries exactly the values needed for
//! `VersionData(N)`.
//!
//! The backfill always runs on a non-readonly DB open. By default it is a
//! dry-run: scan all epoch boundaries, compare derived values with existing
//! entries when present, and print stats. Set `APTOS_VERSION_DATA_BACKFILL` to
//! any non-empty value to switch to write mode, where missing entries are
//! filled in. Existing entries are never overwritten, even on mismatch.

use crate::{
    ledger_db::LedgerDb,
    schema::{epoch_by_version::EpochByVersionSchema, version_data::VersionDataSchema},
};
use aptos_logger::prelude::*;
use aptos_storage_interface::Result;
use aptos_types::{
    account_address::AccountAddress,
    state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage},
    write_set::WriteSet,
};
use move_core_types::{ident_str, language_storage::StructTag};
use serde::Deserialize;
use std::{sync::Arc, thread, time::Instant};

const ENV_VAR: &str = "APTOS_VERSION_DATA_BACKFILL";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    DryRun,
    Write,
}

impl Mode {
    fn from_env() -> Self {
        match std::env::var(ENV_VAR).ok().as_deref() {
            Some(v) if !v.is_empty() => Self::Write,
            _ => Self::DryRun,
        }
    }
}

#[derive(Debug, Default)]
struct Stats {
    epoch_boundaries: u64,
    matched: u64,
    mismatched: u64,
    would_write: u64,
    written: u64,
    skipped_no_writeset: u64,
    skipped_no_resource_write: u64,
    skipped_decode_error: u64,
}

/// Spawn the backfill in a detached background thread. The thread holds an
/// `Arc<LedgerDb>` and runs to completion (single pass over epoch boundaries).
pub(crate) fn spawn(ledger_db: Arc<LedgerDb>) {
    let mode = Mode::from_env();
    info!(
        env_var = ENV_VAR,
        mode = ?mode,
        "Starting `version_data` backfill at epoch boundaries.",
    );
    thread::Builder::new()
        .name("vdata-bckfll".into())
        .spawn(move || {
            if let Err(e) = run(&ledger_db, mode) {
                warn!(error = ?e, "VersionData backfill thread failed.");
            }
        })
        .expect("Failed to spawn version_data backfill thread.");
}

/// `StateKey` for the `0x1::state_storage::StateStorageUsage` resource.
fn resource_key() -> StateKey {
    let tag = StructTag {
        address: AccountAddress::ONE,
        module: ident_str!("state_storage").to_owned(),
        name: ident_str!("StateStorageUsage").to_owned(),
        type_args: vec![],
    };
    StateKey::resource(&AccountAddress::ONE, &tag)
        .expect("StateStorageUsage resource StateKey must construct.")
}

/// BCS layout matches `aptos_framework::state_storage::StateStorageUsage`:
/// `{ epoch: u64, usage: { items: u64, bytes: u64 } }`.
#[derive(Deserialize)]
#[allow(dead_code)]
struct OnChainStateStorageUsage {
    epoch: u64,
    usage: OnChainUsage,
}

#[derive(Deserialize)]
struct OnChainUsage {
    items: u64,
    bytes: u64,
}

/// Decodes the on-chain resource from the write set at the first version of
/// a new epoch, if a write for that key is present.
fn extract_from_writeset(ws: &WriteSet, key: &StateKey) -> Result<Option<StateStorageUsage>> {
    let Some(op) = ws.get_write_op(key) else {
        return Ok(None);
    };
    let Some(value) = op.as_state_value_opt() else {
        // A deletion of this resource would be a framework bug; treat as missing.
        return Ok(None);
    };
    let decoded: OnChainStateStorageUsage = bcs::from_bytes(value.bytes())?;
    Ok(Some(StateStorageUsage::new(
        decoded.usage.items as usize,
        decoded.usage.bytes as usize,
    )))
}

fn run(ledger_db: &LedgerDb, mode: Mode) -> Result<()> {
    let start = Instant::now();
    let key = resource_key();
    let metadata_db = ledger_db.metadata_db();
    let write_set_db = ledger_db.write_set_db();

    let mut stats = Stats::default();

    let mut iter = metadata_db.db().iter::<EpochByVersionSchema>()?;
    iter.seek_to_first();
    for entry in iter {
        let (epoch_end_version, _epoch) = entry?;
        stats.epoch_boundaries += 1;

        let next_version = epoch_end_version + 1;
        let ws = match write_set_db.get_write_set(next_version) {
            Ok(ws) => ws,
            Err(_) => {
                stats.skipped_no_writeset += 1;
                continue;
            },
        };

        let derived = match extract_from_writeset(&ws, &key) {
            Ok(Some(u)) => u,
            Ok(None) => {
                stats.skipped_no_resource_write += 1;
                continue;
            },
            Err(e) => {
                warn!(
                    version = next_version,
                    error = ?e,
                    "Failed to decode StateStorageUsage from write set.",
                );
                stats.skipped_decode_error += 1;
                continue;
            },
        };

        let existing = metadata_db
            .db()
            .get::<VersionDataSchema>(&epoch_end_version)?
            .map(|d| d.get_state_storage_usage());

        match existing {
            Some(prev) if prev == derived => stats.matched += 1,
            Some(prev) => {
                stats.mismatched += 1;
                warn!(
                    version = epoch_end_version,
                    existing_items = prev.items(),
                    existing_bytes = prev.bytes(),
                    derived_items = derived.items(),
                    derived_bytes = derived.bytes(),
                    "VersionData mismatch at epoch-ending version; not overwriting.",
                );
            },
            None => match mode {
                Mode::DryRun => stats.would_write += 1,
                Mode::Write => {
                    metadata_db.put_usage(epoch_end_version, derived)?;
                    stats.written += 1;
                },
            },
        }

        if stats.epoch_boundaries % 10_000 == 0 {
            info!(
                progress = stats.epoch_boundaries,
                elapsed_ms = start.elapsed().as_millis() as u64,
                "VersionData backfill progress.",
            );
        }
    }

    info!(
        mode = ?mode,
        elapsed_ms = start.elapsed().as_millis() as u64,
        epoch_boundaries = stats.epoch_boundaries,
        matched = stats.matched,
        mismatched = stats.mismatched,
        would_write = stats.would_write,
        written = stats.written,
        skipped_no_writeset = stats.skipped_no_writeset,
        skipped_no_resource_write = stats.skipped_no_resource_write,
        skipped_decode_error = stats.skipped_decode_error,
        "VersionData backfill complete.",
    );
    Ok(())
}
