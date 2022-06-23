// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use super::{common::validate_configuration, NodeConfiguration};
use anyhow::{Context, Result};
use clap::{ArgEnum, Parser};
use std::path::PathBuf;

#[derive(ArgEnum, Clone, Debug)]
enum OutputFormat {
    Json,
    Yaml,
}

#[derive(Clone, Debug, Parser)]
pub struct Create {
    #[clap(flatten)]
    node_configuration: NodeConfiguration,

    #[clap(short, long)]
    output_path: Option<PathBuf>,

    #[clap(short, long, arg_enum, default_value = "yaml")]
    format: OutputFormat,

    #[clap(long)]
    do_not_validate: bool,
}

pub async fn create(args: Create) -> Result<()> {
    let output = match args.format {
        OutputFormat::Json => serde_json::to_string_pretty(&args.node_configuration)?,
        OutputFormat::Yaml => serde_yaml::to_string(&args.node_configuration)?,
    };

    if !args.do_not_validate {
        validate_configuration(&args.node_configuration)
            .context("Configuration failed validation")?;
    }

    match args.output_path {
        Some(path) => std::fs::write(path, output)?,
        None => println!("{}", output),
    }

    Ok(())
}
