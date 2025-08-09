// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! A tool to query information from a move packages
//! **Usage**: ` move-querier [OPTIONS] --package_path <PATH_TO_PACKAGE>`
//!
//! - Available `OPTIONS` include (at least one of them must be provided):
//!
//!    - `--dump-call-graph`: Dump the inter-module call graph from the move package as a .dot file.
//!
//!    - `--dump-dep-graph`: Dump the inter-module dependency graph from the move package as a .dot file.
//!
//! `PATH_TO_PACKAGE` can be either a path to an specific bytecode file
//!     or a path to a folder containing a group of bytecode files

use anyhow::Result;
use clap::Parser;
use move_querier::querier::{Querier, QuerierCommand};
use std::{
    fs, io,
    path::{Path, PathBuf},
    process::exit,
};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Arguments {
    #[clap(long, required = true)]
    pub package_path: PathBuf,

    #[clap(flatten)]
    cmd: Option<QuerierCommand>,
}

fn files_with_extension(dir: &Path, ext: &str) -> io::Result<Vec<PathBuf>> {
    let mut matches = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        // only consider files (not directories)
        if path.is_file() {
            // check extension (case‑sensitive)
            if path.extension().and_then(|e| e.to_str()) == Some(ext) {
                matches.push(path);
            }
        }
    }
    Ok(matches)
}

fn main() -> Result<()> {
    let args = Arguments::parse();
    let query_cmd = if let Some(cmds) = args.cmd {
        cmds
    } else {
        println!("Error: no query command provided. For more information, run with --help");
        exit(1);
    };

    let mut querier = Querier::new(query_cmd);
    let package = files_with_extension(&args.package_path, "mv")?;
    querier.load_package(&package, args.package_path)?;
    print!("{}", querier.query()?);
    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Arguments::command().debug_assert()
}
