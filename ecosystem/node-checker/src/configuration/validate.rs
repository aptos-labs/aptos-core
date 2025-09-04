// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{read_configuration_from_file, BaselineConfiguration};
use crate::checker::build_checkers;
use anyhow::{Context, Result};
use velor_logger::debug;
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
