// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Auto-generated, human-reviewable change summaries written under the bundle's
//! `summary/` directory. They give reviewers a quick overview without reading
//! raw Move scripts, and carry sign-off checkboxes (see `verify --require-signoff`).

use crate::table::{self, Align};
use anyhow::{Context, Result};
use aptos_types::on_chain_config::{DiffItem, GasScheduleV2};
use std::{collections::BTreeMap, fmt::Write as _, fs, path::Path};

pub const GAS_SUMMARY: &str = "gas-schedule-changes.md";
pub const FEATURE_FLAG_SUMMARY: &str = "feature-flags.md";

/// Gas parameters whose changes warrant explicit, per-change acknowledgment.
/// Each gets its own checkbox in the summary.
const CRITICAL_GAS_PARAMS: &[&str] = &[
    "txn.max_execution_gas",
    "txn.max_io_gas",
    "txn.max_storage_fee",
    "txn.max_transaction_size_in_bytes",
    "txn.maximum_number_of_gas_units",
    "txn.gas_unit_scaling_factor",
];

/// Write `summary/gas-schedule-changes.md`. When `old` is absent (no previous
/// snapshot to diff against), only the feature version and a note are emitted.
pub fn write_gas_summary(
    summary_dir: &Path,
    old: Option<&GasScheduleV2>,
    new: &GasScheduleV2,
) -> Result<()> {
    let mut out = String::new();

    match old {
        Some(old) => {
            writeln!(out, "# Gas Schedule Changes").ok();
            writeln!(out).ok();
            writeln!(
                out,
                "Gas feature version: {} -> {}",
                old.feature_version, new.feature_version
            )
            .ok();
        },
        None => {
            writeln!(out, "# Gas Schedule").ok();
            writeln!(out).ok();
            writeln!(out, "Gas feature version: {}", new.feature_version).ok();
            writeln!(out).ok();
            writeln!(
                out,
                "_No previous gas schedule was provided, so a parameter diff cannot be shown._"
            )
            .ok();
        },
    }
    writeln!(out).ok();
    writeln!(out, "- [ ] I have reviewed the gas schedule changes below.").ok();
    writeln!(out).ok();

    if let Some(old) = old {
        writeln!(out, "## Changes").ok();
        writeln!(out).ok();

        let changes = GasScheduleV2::diff(old, new);
        if changes.is_empty() {
            writeln!(out, "_No parameter changes._").ok();
        } else {
            emit_gas_change_table(&mut out, &changes);
        }
    }

    let path = summary_dir.join(GAS_SUMMARY);
    fs::write(&path, out).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Render the gas parameter changes as a source-aligned markdown table.
fn emit_gas_change_table(out: &mut String, changes: &BTreeMap<&str, DiffItem<u64>>) {
    let rows: Vec<Vec<String>> = changes
        .iter()
        .map(|(name, delta)| {
            let (change, old_val, new_val) = match delta {
                DiffItem::Modify { old_val, new_val } => {
                    ("modified", old_val.to_string(), new_val.to_string())
                },
                DiffItem::Add { new_val } => ("added", "/".to_string(), new_val.to_string()),
                DiffItem::Delete { old_val } => ("removed", old_val.to_string(), "/".to_string()),
            };
            let signoff = if CRITICAL_GAS_PARAMS.contains(name) {
                "[ ]"
            } else {
                ""
            };
            vec![
                change.to_string(),
                name.to_string(),
                old_val,
                new_val,
                signoff.to_string(),
            ]
        })
        .collect();

    let headers = ["change", "parameter", "old", "new", "sign-off"];
    let aligns = [
        Align::Left,
        Align::Left,
        Align::Right,
        Align::Right,
        Align::Left,
    ];
    out.push_str(&table::render(&headers, &aligns, &rows));
}

/// Write `summary/feature-flags.md`. When there are no feature flag changes, a
/// note is written and no sign-off checkbox is emitted (nothing to review).
pub fn write_feature_flag_summary(
    summary_dir: &Path,
    enabled: &[String],
    disabled: &[String],
) -> Result<()> {
    let mut out = String::new();
    writeln!(out, "# Feature Flag Changes").ok();
    writeln!(out).ok();

    if enabled.is_empty() && disabled.is_empty() {
        writeln!(out, "_No feature flag changes in this release._").ok();
    } else {
        writeln!(out, "- [ ] I have reviewed the feature flag changes below.").ok();
        writeln!(out).ok();

        if !enabled.is_empty() {
            writeln!(out, "## Enabling").ok();
            writeln!(out).ok();
            for flag in enabled {
                writeln!(out, "- {}", flag).ok();
            }
            writeln!(out).ok();
        }
        if !disabled.is_empty() {
            writeln!(out, "## Disabling").ok();
            writeln!(out).ok();
            for flag in disabled {
                writeln!(out, "- {}", flag).ok();
            }
            writeln!(out).ok();
        }
    }

    let path = summary_dir.join(FEATURE_FLAG_SUMMARY);
    fs::write(&path, out).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

/// Scan a summary file's contents for unchecked checkboxes (`[ ]`). Used by
/// `verify --require-signoff`.
pub fn has_unchecked_boxes(contents: &str) -> bool {
    contents.contains("[ ]")
}

/// Count `(ticked, total)` checkboxes in a summary file's contents.
pub fn box_counts(contents: &str) -> (usize, usize) {
    let ticked = contents.matches("[x]").count() + contents.matches("[X]").count();
    let unchecked = contents.matches("[ ]").count();
    (ticked, ticked + unchecked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use move_command_line_common::testing::read_env_update_baseline;
    use std::path::{Path, PathBuf};

    fn test_data_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/data")
    }

    fn load_gas(name: &str) -> GasScheduleV2 {
        let path = test_data_dir().join(name);
        let contents =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {}", path.display(), e));
        serde_json::from_str(&contents)
            .unwrap_or_else(|e| panic!("parse {}: {}", path.display(), e))
    }

    /// Compare `actual` against the committed golden, or rewrite the golden when
    /// the repo's baseline-update env var is set (`UPDATE_BASELINE=1`, `UB=1`):
    ///
    ///   UB=1 cargo test -p aptos-release-tool --lib
    fn assert_or_update_golden(golden_path: &Path, actual: &str) {
        if read_env_update_baseline() {
            fs::write(golden_path, actual)
                .unwrap_or_else(|e| panic!("write golden {}: {}", golden_path.display(), e));
            return;
        }
        let expected = fs::read_to_string(golden_path).unwrap_or_else(|e| {
            panic!(
                "read golden {} (set UB=1 to create it): {}",
                golden_path.display(),
                e
            )
        });
        assert_eq!(
            actual,
            expected,
            "golden {} is stale; re-run with UB=1 to update",
            golden_path.display()
        );
    }

    /// `write_gas_summary` renders the synthetic modify/add/remove diff exactly
    /// as the committed golden. This is pure (no framework compilation), so it
    /// stays green even when a framework change breaks the end-to-end test, and
    /// is the fast entry point for updating the golden (see above).
    #[test]
    fn gas_summary_matches_golden() {
        let old = load_gas("gas_old.json");
        let new = load_gas("gas_new.json");

        let dir = tempfile::tempdir().expect("tempdir");
        write_gas_summary(dir.path(), Some(&old), &new).expect("write summary");
        let actual = fs::read_to_string(dir.path().join(GAS_SUMMARY)).expect("read summary");

        assert_or_update_golden(&test_data_dir().join("expected_gas_summary.md"), &actual);
    }
}
