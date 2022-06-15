// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;
use log::debug;

use super::read_configuration_from_file;

#[derive(Clone, Debug, Parser)]
pub struct Validate {
    #[clap(short, long)]
    path: PathBuf,
}

pub async fn validate(args: Validate) -> Result<()> {
    let configuration = read_configuration_from_file(args.path)?;
    debug!("{:#?}", configuration);
    Ok(())
}
