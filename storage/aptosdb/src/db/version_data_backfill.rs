// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Backfills `VersionData` rows at epoch-ending versions by deriving the
//! values from the next version's write set.
//!
//! Background: the prologue of the first transaction of every new epoch
//! invokes the `0x1::state_storage::get_state_storage_usage_only_at_epoch_beginning`
//! native and writes the result into the on-chain
//! `0x1::state_storage::StateStorageUsage` resource. That on-chain value is
//! exactly what should live in `VersionData[v_end]`, where
//! `v_end = v_start - 1` and `v_start` is the first version of the new epoch.
//!
//! So for any epoch-ending version we can reconstruct the (missing) row by
//! reading `write_set_db[v_start]`, finding the modification for the
//! `StateStorageUsage` resource, and BCS-decoding `(epoch, items, bytes)`.
//!
//! Enable / disable via the two `ENABLE_*` constants below.

use crate::{db::AptosDB, ledger_db::LedgerDb};
use aptos_logger::prelude::*;
use aptos_storage_interface::{AptosDbError, Result};
use aptos_types::{
    state_store::{state_key::StateKey, state_storage_usage::StateStorageUsage},
    write_set::WriteSet,
};
use move_core_types::{
    identifier::Identifier,
    language_storage::{StructTag, CORE_CODE_ADDRESS},
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Instant};

// === FEATURE FLAGS ==========================================================
//
// Both flags default to `false`. Flip ENABLE_BACKGROUND_THREAD to spawn the
// scan on AptosDB::open; flip ENABLE_REAL_RUN to also persist the derived
// rows after the dry-run. AptosDB::run_version_data_backfill_{dry_run,real}
// can be invoked directly regardless of these flags.
// =============================================================================

/// Spawn the backfill scan in a background thread when `AptosDB::open` runs.
/// Dry-run only by default; flip `ENABLE_REAL_RUN` to also write.
pub(crate) const ENABLE_BACKGROUND_THREAD: bool = false;

/// Actually write the derived rows after the dry-run completes. Only effective
/// inside the background thread (or callers of `run_version_data_backfill_real`).
pub(crate) const ENABLE_REAL_RUN: bool = false;

/// How often to emit a progress line during the scan.
const PROGRESS_LOG_INTERVAL_SECS: u64 = 15;
/// Cap on the number of mismatching versions to log individually before we go
/// silent on the per-row warnings (the aggregate stat still tracks all of them).
const MAX_MISMATCH_LOG_LINES: u64 = 32;
/// Same cap for "no StateStorageUsage write op found" warnings.
const MAX_FAILED_LOOKUP_LOG_LINES: u64 = 32;

/// BCS-decodable mirror of the on-chain `0x1::state_storage::StateStorageUsage`
/// resource. Move struct: `{ epoch: u64, usage: Usage { items: u64, bytes: u64 } }`.
#[derive(Debug, Deserialize, Serialize)]
struct OnChainUsage {
    items: u64,
    bytes: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct OnChainStateStorageUsage {
    epoch: u64,
    usage: OnChainUsage,
}

#[derive(Debug, Default, Clone)]
pub struct BackfillStats {
    /// Number of epoch-ending LedgerInfos visited.
    pub epochs_scanned: u64,
    /// Existing row matches the value derived from the next write set.
    pub already_present_matches: u64,
    /// Existing row exists but disagrees with the value derived from the
    /// write set (or is `is_untracked`). These are logged individually up to
    /// MAX_MISMATCH_LOG_LINES.
    pub already_present_mismatch: u64,
    /// No existing row; backfill would (or did) write a fresh value.
    pub would_write: u64,
    /// Actually persisted (only non-zero in real-run mode).
    pub wrote: u64,
    /// v_end is at or beyond the synced tip, so v_end+1 doesn't exist.
    pub skipped_tip: u64,
    /// Couldn't read the write set at v_end+1, e.g., pruned.
    pub failed_write_set_read: u64,
    /// Write set at v_end+1 didn't contain a StateStorageUsage modification.
    pub failed_lookup: u64,
    /// Other unexpected errors (BCS decode, iterator, put_usage, etc.).
    pub other_errors: u64,
}

impl BackfillStats {
    fn log_summary(&self, dry_run: bool, elapsed_secs: f64) {
        info!(
            "[vd-backfill] {} done in {:.1}s: epochs_scanned={} already_match={} \
             would_write={} wrote={} mismatch={} failed_ws_read={} failed_lookup={} \
             skipped_tip={} other_errors={}",
            if dry_run { "DRY-RUN" } else { "REAL" },
            elapsed_secs,
            self.epochs_scanned,
            self.already_present_matches,
            self.would_write,
            self.wrote,
            self.already_present_mismatch,
            self.failed_write_set_read,
            self.failed_lookup,
            self.skipped_tip,
            self.other_errors,
        );
    }
}

fn state_storage_usage_state_key() -> Result<StateKey> {
    let struct_tag = StructTag {
        address: CORE_CODE_ADDRESS,
        module: Identifier::new("state_storage").map_err(other_err)?,
        name: Identifier::new("StateStorageUsage").map_err(other_err)?,
        type_args: vec![],
    };
    StateKey::resource(&CORE_CODE_ADDRESS, &struct_tag).map_err(other_err)
}

fn other_err<E: std::fmt::Display>(e: E) -> AptosDbError {
    AptosDbError::Other(e.to_string())
}

/// Parse the `StateStorageUsage` value out of a write set, if present.
fn derive_usage_from_write_set(
    ws: &WriteSet,
    target_key: &StateKey,
) -> Result<Option<StateStorageUsage>> {
    let Some(bytes) = ws
        .write_op_iter()
        .find_map(|(k, op)| (k == target_key).then_some(op).and_then(|op| op.bytes()))
    else {
        return Ok(None);
    };
    let on_chain: OnChainStateStorageUsage = bcs::from_bytes(bytes).map_err(other_err)?;
    Ok(Some(StateStorageUsage::new(
        on_chain.usage.items as usize,
        on_chain.usage.bytes as usize,
    )))
}

/// Core scan loop. Iterates every epoch-ending version and, depending on
/// `write`, either reports what would happen or persists the derived row.
fn run(ledger_db: &LedgerDb, write: bool) -> Result<BackfillStats> {
    let start = Instant::now();
    info!("[vd-backfill] starting scan (dry_run={})", !write);

    let metadata_db = ledger_db.metadata_db();
    let write_set_db = ledger_db.write_set_db();
    let target_key = state_storage_usage_state_key()?;

    let Some(latest_li) = metadata_db.get_latest_ledger_info_option() else {
        info!("[vd-backfill] DB has no LedgerInfos; nothing to do");
        return Ok(BackfillStats::default());
    };
    let latest_version = latest_li.ledger_info().version();
    info!("[vd-backfill] latest synced version = {}", latest_version);

    let iter = metadata_db.get_epoch_ending_ledger_info_iter(0, u64::MAX)?;
    let mut stats = BackfillStats::default();
    let mut last_progress_log = Instant::now();
    let mut mismatch_logs_emitted = 0u64;
    let mut failed_lookup_logs_emitted = 0u64;

    for li_result in iter {
        let li = match li_result {
            Ok(li) => li,
            Err(e) => {
                warn!("[vd-backfill] LI iterator error: {}", e);
                stats.other_errors += 1;
                continue;
            },
        };
        let v_end = li.ledger_info().version();
        stats.epochs_scanned += 1;

        if last_progress_log.elapsed().as_secs() >= PROGRESS_LOG_INTERVAL_SECS {
            info!(
                "[vd-backfill] progress: epochs_scanned={} at v_end={} ({:.2}% of tip)",
                stats.epochs_scanned,
                v_end,
                100.0 * v_end as f64 / (latest_version.max(1) as f64),
            );
            last_progress_log = Instant::now();
        }

        // The tip itself may be exactly an epoch-ending version (e.g., a
        // node that just rolled over an epoch and hasn't committed v+1 yet).
        // We need v_end + 1 to derive the row, so skip in that case.
        if v_end >= latest_version {
            stats.skipped_tip += 1;
            continue;
        }

        let v_start = v_end + 1;
        let ws = match write_set_db.get_write_set(v_start) {
            Ok(ws) => ws,
            Err(e) => {
                stats.failed_write_set_read += 1;
                if stats.failed_write_set_read <= 4 {
                    warn!(
                        "[vd-backfill] write_set read failed at v={} (epoch_end={}): {}",
                        v_start, v_end, e,
                    );
                }
                continue;
            },
        };

        let derived = match derive_usage_from_write_set(&ws, &target_key) {
            Ok(Some(u)) => u,
            Ok(None) => {
                stats.failed_lookup += 1;
                if failed_lookup_logs_emitted < MAX_FAILED_LOOKUP_LOG_LINES {
                    warn!(
                        "[vd-backfill] no StateStorageUsage write op in write_set at \
                         v={} (epoch_end={})",
                        v_start, v_end,
                    );
                    failed_lookup_logs_emitted += 1;
                }
                continue;
            },
            Err(e) => {
                stats.other_errors += 1;
                warn!(
                    "[vd-backfill] decode StateStorageUsage at v={} failed: {}",
                    v_start, e,
                );
                continue;
            },
        };

        match metadata_db.get_usage(v_end) {
            Ok(existing) => {
                let same = !existing.is_untracked()
                    && existing.items() == derived.items()
                    && existing.bytes() == derived.bytes();
                if same {
                    stats.already_present_matches += 1;
                } else {
                    stats.already_present_mismatch += 1;
                    if mismatch_logs_emitted < MAX_MISMATCH_LOG_LINES {
                        warn!(
                            "[vd-backfill] MISMATCH at v_end={} (epoch end): existing \
                             items={} bytes={} untracked={} vs derived items={} bytes={}",
                            v_end,
                            existing.items(),
                            existing.bytes(),
                            existing.is_untracked(),
                            derived.items(),
                            derived.bytes(),
                        );
                        mismatch_logs_emitted += 1;
                    }
                }
            },
            Err(_) => {
                // Missing or corrupt — this is the case we want to backfill.
                stats.would_write += 1;
                if write {
                    if let Err(e) = metadata_db.put_usage(v_end, derived) {
                        stats.other_errors += 1;
                        warn!("[vd-backfill] put_usage failed at v_end={}: {}", v_end, e,);
                    } else {
                        stats.wrote += 1;
                    }
                }
            },
        }
    }

    stats.log_summary(!write, start.elapsed().as_secs_f64());
    Ok(stats)
}

impl AptosDB {
    /// Scan every epoch-ending version and report what would be written. Read-only.
    pub fn run_version_data_backfill_dry_run(&self) -> Result<BackfillStats> {
        run(&self.ledger_db, /* write = */ false)
    }

    /// Same as the dry-run, but actually persists derived `VersionData` rows
    /// for epoch endings that lack one. Wrap or remove the call site to
    /// enable / disable real writes.
    pub fn run_version_data_backfill_real(&self) -> Result<BackfillStats> {
        run(&self.ledger_db, /* write = */ true)
    }

    /// Conditionally spawn a background thread that performs the dry-run and
    /// optionally the real run. Gated entirely on the module-level
    /// `ENABLE_BACKGROUND_THREAD` flag; no-op otherwise.
    pub(super) fn maybe_spawn_version_data_backfill_thread(&self) {
        if !ENABLE_BACKGROUND_THREAD {
            return;
        }
        let ledger_db = Arc::clone(&self.ledger_db);
        let builder = std::thread::Builder::new().name("vd-backfill".to_string());
        let spawn_result = builder.spawn(move || {
            let dry = match run(&ledger_db, /* write = */ false) {
                Ok(s) => s,
                Err(e) => {
                    warn!("[vd-backfill] dry-run failed: {}", e);
                    return;
                },
            };
            if !ENABLE_REAL_RUN {
                info!(
                    "[vd-backfill] ENABLE_REAL_RUN=false; dry-run finished, skipping real \
                     write (would_write={})",
                    dry.would_write,
                );
                return;
            }
            match run(&ledger_db, /* write = */ true) {
                Ok(_) => {},
                Err(e) => warn!("[vd-backfill] real run failed: {}", e),
            }
        });
        if let Err(e) = spawn_result {
            warn!("[vd-backfill] failed to spawn backfill thread: {}", e);
        }
    }
}
