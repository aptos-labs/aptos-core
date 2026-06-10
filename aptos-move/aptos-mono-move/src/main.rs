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
use aptos_mono_move::{cache::FlatState, compare, dump::Dump, txn, v1, v2};
use clap::{Parser, ValueEnum};
use mono_move_global_context::{ExecutionGuard, GlobalContext};

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
}

enum Outcome {
    Ran { mismatched: bool },
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

    let mut ran = 0u64;
    let mut skipped = 0u64;
    let mut mismatched_txns = 0u64;

    for version in versions {
        match run_one(&dump, &guard, args.vm, version) {
            Ok(Outcome::Ran { mismatched }) => {
                ran += 1;
                if mismatched {
                    mismatched_txns += 1;
                }
            },
            Ok(Outcome::Skipped) => skipped += 1,
            Err(err) => {
                eprintln!("v{version}: skip: {err}");
                skipped += 1;
            },
        }
        // Blank line between transactions for readability.
        println!();
    }

    println!("summary: ran={ran} skipped={skipped} mismatched_txns={mismatched_txns}");
    Ok(())
}

fn run_one(dump: &Dump, guard: &ExecutionGuard, vm: Vm, version: u64) -> Result<Outcome> {
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
            let outcome = v1::run(&flat, &raw_state, signed, aux_info, &entry)?;
            println!(
                "  v1: {:?} writes={}",
                outcome.elapsed,
                outcome.writes.len(),
            );
            print_abort("v1", &outcome.abort_reason);
            Ok(Outcome::Ran { mismatched: false })
        },
        Vm::Both => {
            let v1_outcome = v1::run(&flat, &raw_state, signed, aux_info, &entry)?;
            println!(
                "  v1: {:?} writes={}",
                v1_outcome.elapsed,
                v1_outcome.writes.len(),
            );
            print_abort("v1", &v1_outcome.abort_reason);

            let layout = match &v1_outcome.layout {
                Ok(layout) => layout,
                Err(reason) => {
                    println!("  v2: skipped ({reason})");
                    return Ok(Outcome::Ran { mismatched: false });
                },
            };
            let keys: Vec<_> = v1_outcome.writes.keys().cloned().collect();
            let v2_outcome = v2::run(guard, &flat, &entry, layout, &keys)?;
            println!("  v2: {:?}", v2_outcome.elapsed);
            print_abort("v2", &v2_outcome.abort_reason);

            let diff = compare::compare(&v1_outcome, &v2_outcome);
            let mismatched = diff.mismatched > 0 || diff.missing > 0;
            println!(
                "  resources: {} match, {} mismatch, {} missing",
                diff.matched, diff.mismatched, diff.missing,
            );
            Ok(Outcome::Ran { mismatched })
        },
    }
}

/// Prints an indented abort line for `vm` when it aborted/errored.
fn print_abort(vm: &str, reason: &Option<String>) {
    if let Some(reason) = reason {
        println!("  {vm} abort: {reason}");
    }
}
