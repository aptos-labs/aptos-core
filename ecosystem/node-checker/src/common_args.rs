// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::{ArgEnum, Parser};
use std::path::PathBuf;

#[derive(ArgEnum, Clone, Debug)]
pub enum OutputFormat {
    Json,
    Yaml,
}

#[derive(Clone, Debug, Parser)]
pub struct OutputArgs {
    /// By default, the spec is written to stdout. If this is provided, the
    /// tool will instead write the spec to the provided path.
    #[clap(long)]
    pub output_path: Option<PathBuf>,

    /// What format to output the spec in.
    #[clap(long, arg_enum, default_value = "yaml")]
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
