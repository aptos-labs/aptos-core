// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use std::path::PathBuf;

use anyhow::Result;
use clap::{ArgEnum, Parser};

use super::NodeConfiguration;

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
}

pub async fn create(args: Create) -> Result<()> {
    let s = match args.format {
        OutputFormat::Json => serde_json::to_string_pretty(&args.node_configuration)?,
        OutputFormat::Yaml => serde_yaml::to_string(&args.node_configuration)?,
    };

    match args.output_path {
        Some(path) => std::fs::write(path, s)?,
        None => println!("{}", s),
    }

    Ok(())
}
