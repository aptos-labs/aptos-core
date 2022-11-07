// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct GenArgs {
    #[clap(short, long)]
    pub output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = GenArgs::parse();

    // TODO: Being able to parse the release config from a TOML file to generate the proposals.
    aptos_release_builder::ReleaseConfig::default()
        .generate_release_proposal_scripts(args.output.as_ref().unwrap(), true)
}
