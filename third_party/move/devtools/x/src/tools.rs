// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{context::XContext, Result};
use anyhow::anyhow;
use clap::Parser;

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(long)]
    /// Run in 'check' mode. Exits with 0 if all tools installed. Exits with 1 and if not, printing failed
    check: bool,
}

pub fn run(args: Args, xctx: XContext) -> Result<()> {
    let success = match args.check {
        false => xctx.installer().install_all(),
        true => xctx.installer().check_all(),
    };
    if success {
        Ok(())
    } else {
        Err(anyhow!("Failed to install tools"))
    }
}
