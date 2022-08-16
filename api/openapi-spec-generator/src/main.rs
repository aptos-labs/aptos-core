// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

mod fake_context;

use anyhow::Result;
use aptos_api::get_api_service;
use clap::{ArgEnum, Parser};
use fake_context::get_fake_context;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(ArgEnum, Clone, Debug)]
pub enum OutputFormat {
    Json,
    Yaml,
}

#[derive(Clone, Debug, Parser)]
pub struct OutputArgs {
    /// By default, the spec is written to stdout. If this is provided, the
    /// tool will instead write the spec to the provided path.
    #[clap(short, long)]
    pub output_path: Option<PathBuf>,

    /// What format to output the spec in.
    #[clap(short, long, arg_enum, default_value = "yaml")]
    pub format: OutputFormat,
}

impl OutputArgs {
    pub fn write(&self, output: &str) -> Result<()> {
        match &self.output_path {
            Some(path) => std::fs::write(path, output)?,
            None => println!("{}", output),
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Parser)]
pub struct Args {
    #[clap(flatten)]
    pub output_args: OutputArgs,
}

pub fn main() -> Result<()> {
    let args = Args::parse();

    let api_service = get_api_service(Arc::new(get_fake_context()));

    let spec = match args.output_args.format {
        OutputFormat::Json => api_service.spec(),
        OutputFormat::Yaml => api_service.spec_yaml(),
    };
    args.output_args.write(&spec)
}
