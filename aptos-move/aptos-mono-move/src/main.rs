// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! CLI: replay dumped transactions on MonoMove and the legacy MoveVM.
//!
//! By default it runs both VMs on every transaction in the dump and compares
//! their global-storage writes. `--vm v1` runs only the legacy MoveVM (to smoke
//! a transaction in isolation), and `--version` restricts to specific
//! transactions.
//!
//! Usage:
//!   aptos-mono-move <dump-dir> [--vm v1|both] [--version <V>]...
//!
//! Example:
//!   aptos-mono-move aptos-move/aptos-mono-move/data --vm v1 --version 5664072623

use anyhow::{anyhow, Result};
use aptos_mono_move::{cache::FlatState, compare, dump::Dump, events, txn, v1, v2};
use clap::{Parser, ValueEnum};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use std::time::Duration;

#[derive(Clone, Copy, ValueEnum)]
enum Vm {
    /// Legacy MoveVM only.
    V1,
    /// Both VMs, comparing their writes.
    Both,
}

#[derive(Parser)]
#[command(about = "Replay dumped transactions on MonoMove and the legacy MoveVM")]
struct Args {
    /// Dump directory (e.g. aptos-move/aptos-mono-move/data).
    dump_dir: String,
    /// Which VM(s) to run.
    #[arg(long, value_enum, default_value = "both")]
    vm: Vm,
    /// Restrict to these transaction versions (repeatable). Default: all
    /// versions in the dump.
    #[arg(long = "version")]
    versions: Vec<u64>,
    /// Enable paranoid runtime type checks in the legacy MoveVM (V1). Pass
    /// `--paranoid-type-checks false` to disable. Defaults to on, matching the
    /// production VM.
    #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
    paranoid_type_checks: bool,
}

enum Outcome {
    /// Both VMs reached a comparable conclusion (both succeeded and were
    /// compared, or both aborted). `matched` is whether they agreed.
    Compared { matched: bool },
    /// V2 could not be run, e.g. an argument type it does not support yet, so
    /// there is nothing to compare against V1.
    Incomparable,
    /// Ran the legacy VM only (`--vm v1`); no comparison performed.
    RanV1Only,
    /// Not a comparable transaction (script / multisig / not found in dump).
    Skipped,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let dump = Dump::open(&args.dump_dir)?;
    let versions = if args.versions.is_empty() {
        dump.versions()?
    } else {
        args.versions.clone()
    };

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .ok_or_else(|| anyhow!("failed to acquire execution guard"))?;

    let mut matched = 0u64;
    let mut mismatched = 0u64;
    let mut incomparable = 0u64;
    let mut v1_only = 0u64;
    let mut skipped = 0u64;

    for version in versions {
        match run_one(&dump, &guard, args.vm, version, args.paranoid_type_checks) {
            Ok(Outcome::Compared { matched: true }) => matched += 1,
            Ok(Outcome::Compared { matched: false }) => mismatched += 1,
            Ok(Outcome::Incomparable) => incomparable += 1,
            Ok(Outcome::RanV1Only) => v1_only += 1,
            Ok(Outcome::Skipped) => skipped += 1,
            Err(err) => {
                eprintln!("v{version}: skip: {err}");
                skipped += 1;
            },
        }
        // Blank line between transactions for readability.
        println!();
    }

    match args.vm {
        Vm::V1 => println!("summary: ran={v1_only} skipped={skipped}"),
        Vm::Both => println!(
            "summary: compared={} matched={matched} mismatched={mismatched} \
             incomparable={incomparable} skipped={skipped}",
            matched + mismatched,
        ),
    }
    Ok(())
}

fn run_one(
    dump: &Dump,
    guard: &ExecutionGuard,
    vm: Vm,
    version: u64,
    paranoid_type_checks: bool,
) -> Result<Outcome> {
    let Some(transaction) = dump.transaction(version)? else {
        eprintln!("v{version}: not found in dump");
        return Ok(Outcome::Skipped);
    };
    let Some(entry) = txn::entry_call(&transaction) else {
        eprintln!("v{version}: not a single-signer entry-function txn");
        return Ok(Outcome::Skipped);
    };
    let Some(signed) = txn::signed_user_txn(&transaction) else {
        eprintln!("v{version}: not a user transaction");
        return Ok(Outcome::Skipped);
    };
    let raw_state = dump.state(version)?;
    let aux_info = dump.aux_info(version)?;
    let flat = FlatState::build(&raw_state)?;
    let name = format!("{}::{}", entry.module.name(), entry.function);

    println!("v{version} {name}");
    match vm {
        Vm::V1 => {
            let outcome =
                v1::run(&flat, &raw_state, signed, aux_info, &entry, paranoid_type_checks)?;
            let changes =
                outcome.writes.len() + outcome.table_writes.len() + outcome.events.len();
            print_status(
                "v1",
                outcome.elapsed,
                outcome.aborted,
                &outcome.abort_reason,
                changes,
            );
            Ok(Outcome::RanV1Only)
        },
        Vm::Both => {
            let v1_outcome =
                v1::run(&flat, &raw_state, signed, aux_info, &entry, paranoid_type_checks)?;
            let v1_changes = v1_outcome.writes.len()
                + v1_outcome.table_writes.len()
                + v1_outcome.events.len();
            print_status(
                "v1",
                v1_outcome.elapsed,
                v1_outcome.aborted,
                &v1_outcome.abort_reason,
                v1_changes,
            );

            let layout = match &v1_outcome.layout {
                Ok(layout) => layout,
                Err(reason) => {
                    println!("  v2: skipped ({reason})");
                    return Ok(Outcome::Incomparable);
                },
            };
            let keys: Vec<_> = v1_outcome.writes.keys().cloned().collect();
            let v2_outcome = v2::run(guard, &flat, signed, aux_info, &entry, layout, &keys)?;
            // V2's resource writes are queried over V1's keys, so this counts
            // how many of those keys V2 also wrote, plus its table writes and
            // events.
            let v2_changes = v2_outcome.writes.values().filter(|w| w.is_some()).count()
                + v2_outcome.table_writes.len()
                + v2_outcome.events.len();
            print_status(
                "v2",
                v2_outcome.elapsed,
                v2_outcome.aborted,
                &v2_outcome.abort_reason,
                v2_changes,
            );

            // If the VMs disagree on succeed-vs-abort, that disagreement is the
            // divergence; the per-resource/event comparison below only makes
            // sense when both succeeded (an aborted run produces no writes).
            match (v1_outcome.aborted, v2_outcome.aborted) {
                (true, true) => {
                    println!("  => both aborted (agree)");
                    return Ok(Outcome::Compared { matched: true });
                },
                (false, true) | (true, false) => {
                    println!(
                        "  => DIVERGED: v1 {}, v2 {}",
                        status_word(v1_outcome.aborted),
                        status_word(v2_outcome.aborted),
                    );
                    return Ok(Outcome::Compared { matched: false });
                },
                (false, false) => {},
            }

            // Both succeeded: compare the writes they made and the events they
            // emitted.
            let diff = compare::compare(&v1_outcome, &v2_outcome);
            println!(
                "  resources: {} match, {} mismatch, {} only-in-v1 (v2 did not write)",
                diff.matched, diff.mismatched, diff.missing,
            );

            let event_diff =
                events::compare_events(guard, &v1_outcome.events, &v2_outcome.events);
            println!(
                "  events: {} match, {} mismatch (v1 emitted {}, v2 emitted {})",
                event_diff.matched, event_diff.mismatched, event_diff.v1_count, event_diff.v2_count,
            );
            for reason in &event_diff.mismatches {
                println!("    {reason}");
            }

            let table_diff = compare::compare_table_writes(&v1_outcome, &v2_outcome);
            println!(
                "  table items: {} match, {} mismatch, {} only-in-v1, {} only-in-v2",
                table_diff.matched,
                table_diff.mismatched,
                table_diff.only_in_v1,
                table_diff.only_in_v2,
            );

            let matched = diff.mismatched == 0
                && diff.missing == 0
                && event_diff.mismatched == 0
                && table_diff.mismatched == 0
                && table_diff.only_in_v1 == 0
                && table_diff.only_in_v2 == 0;
            Ok(Outcome::Compared { matched })
        },
    }
}

/// Prints a VM's one-line status: `ok` with its total change count (resource
/// writes + table-item writes + events), or `ABORTED` with the abort reason,
/// plus the wall-clock time.
fn print_status(
    vm: &str,
    elapsed: Duration,
    aborted: bool,
    abort_reason: &Option<String>,
    changes: usize,
) {
    if aborted {
        let reason = abort_reason.as_deref().unwrap_or("aborted");
        println!("  {vm}: ABORTED ({reason})  {elapsed:?}");
    } else {
        println!("  {vm}: ok  changes={changes}  {elapsed:?}");
    }
}

/// "succeeded" / "aborted" for the divergence headline.
fn status_word(aborted: bool) -> &'static str {
    if aborted {
        "aborted"
    } else {
        "succeeded"
    }
}
