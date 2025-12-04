// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::{read_configuration_from_file, BaselineConfiguration};
use crate::checker::build_checkers;
use anyhow::{Context, Result};
use aptos_logger::debug;
use clap::Parser;
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

pub fn validate_configuration(node_configuration: &BaselineConfiguration) -> Result<()> {
    build_checkers(&node_configuration.checkers).context("Failed to build Checkers")?;
    Ok(())
}
