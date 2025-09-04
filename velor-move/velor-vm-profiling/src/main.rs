// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Parser;
use once_cell::sync::Lazy;
use std::path::Path;

mod profile_velor_vm;
mod profile_move_vm;
mod valgrind;

const BUILD_PROFILE: &str = "release";
static PATH_CRATE_ROOT: Lazy<&Path> = Lazy::new(|| Path::new(env!("CARGO_MANIFEST_DIR")));

#[derive(Parser, Debug)]
struct Args {
    #[clap(short = 'r', long)]
    regenerate_all: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    profile_move_vm::run(args.regenerate_all)?;
    profile_velor_vm::run()?;

    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
