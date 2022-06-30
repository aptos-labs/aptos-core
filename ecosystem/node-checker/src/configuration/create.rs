// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{common::validate_configuration, NodeConfiguration};
use crate::common_args::{OutputArgs, OutputFormat};
use anyhow::{Context, Result};
use clap::Parser;

#[derive(Clone, Debug, Parser)]
pub struct Create {
    #[clap(flatten)]
    node_configuration: NodeConfiguration,

    #[clap(flatten)]
    output_args: OutputArgs,

    // If set, skip config validation. Use with great care.
    #[clap(long)]
    skip_validation: bool,
}

pub async fn create(args: Create) -> Result<()> {
    if !args.skip_validation {
        validate_configuration(&args.node_configuration)
            .context("Configuration failed validation")?;
    }
    let output = match args.output_args.format {
        OutputFormat::Json => serde_json::to_string_pretty(&args.node_configuration)?,
        OutputFormat::Yaml => {
            let mut output = format!(
                "# Base config generated with: {}\n",
                std::env::args().collect::<Vec<_>>().join(" ")
            );
            output.push_str(&serde_yaml::to_string(&args.node_configuration)?);
            output
        }
    };
    args.output_args.write(&output)
}
