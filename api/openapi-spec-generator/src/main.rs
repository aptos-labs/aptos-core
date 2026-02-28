// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
    #[clap(short, long, ignore_case = true, value_enum, default_value_t = OutputFormat::Yaml)]
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

    let spec = match args.output_args.format {
        OutputFormat::Json => include_str!("../../doc/spec.json"),
        OutputFormat::Yaml => include_str!("../../doc/spec.yaml"),
    };

    args.output_args.write(spec)
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
