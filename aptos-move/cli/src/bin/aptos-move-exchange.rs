// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Lightweight single-file entrypoint for the Move exchange frontend.
//!
//! This is intentionally narrower than `aptos move exchange`: it implements
//! only the `--masm-file` and `--move-file` modes used by Lean elaborators.
//! Keeping it separate avoids building the full Aptos CLI for Lean tests.

#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use aptos_move_cli::exchange;
use clap::{ArgGroup, Parser};
use std::{path::PathBuf, process::ExitCode};

#[derive(Parser)]
#[command(
    name = "aptos-move-exchange",
    about = "Export one Move source file in the Lean exchange format",
    group(ArgGroup::new("input").required(true).args(["masm_file", "move_file"]))
)]
struct Args {
    /// A single masm file to export.
    #[arg(long, value_name = "PATH")]
    masm_file: Option<PathBuf>,

    /// A single self-contained Move module to export.
    #[arg(long, value_name = "PATH")]
    move_file: Option<PathBuf>,

    /// File to receive the exchange JSON.
    #[arg(long, value_name = "PATH")]
    out_file: PathBuf,
}

fn run(args: Args) -> Result<()> {
    let module = match (args.masm_file, args.move_file) {
        (Some(path), None) => {
            let input = std::fs::read_to_string(&path)
                .with_context(|| format!("cannot read masm input {}", path.display()))?;
            exchange::masm_to_module(&input)
        },
        (None, Some(path)) => exchange::move_file_to_module(&path),
        _ => unreachable!("clap enforces exactly one input"),
    }
    .context("could not produce the Move exchange module")?;

    std::fs::write(&args.out_file, module.to_pretty_json())
        .with_context(|| format!("cannot write exchange output {}", args.out_file.display()))
}

fn main() -> ExitCode {
    match run(Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err:#}");
            ExitCode::FAILURE
        },
    }
}
