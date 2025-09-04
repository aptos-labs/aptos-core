// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(ValueEnum, Clone, Debug)]
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
    #[clap(short, long, value_enum, ignore_case = true, default_value_t = OutputFormat::Yaml)]
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
