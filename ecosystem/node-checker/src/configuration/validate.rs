// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{common::validate_configuration, read_configuration_from_file};
use anyhow::{Context, Result};
use clap::Parser;
use log::debug;
use std::path::PathBuf;

#[derive(Clone, Debug, Parser)]
pub struct Validate {
    #[clap(short, long)]
    path: PathBuf,
}

pub async fn validate(args: Validate) -> Result<()> {
    let configuration = read_configuration_from_file(args.path)?;
    validate_configuration(&configuration).context("Configuration failed validation")?;
    debug!("Validated configuration: {:#?}", configuration);
    Ok(())
}
