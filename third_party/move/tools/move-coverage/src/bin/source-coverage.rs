// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::Context;
use clap::Parser;
use move_binary_format::file_format::CompiledModule;
use move_bytecode_source_map::utils::source_map_from_file;
use move_command_line_common::files::SOURCE_MAP_EXTENSION;
use move_coverage::{
    coverage_map::CoverageMap,
    source_coverage::{ColorChoice, SourceCoverageBuilder, TextIndicator},
};
use std::{
    fs,
    fs::File,
    io::{self, Write},
    path::Path,
};

#[derive(Debug, Parser)]
#[clap(
    name = "source-coverage",
    about = "Annotate Move Source Code with Coverage Information",
    author,
    version
)]
struct Args {
    /// The path to the coverage map or trace file
    #[clap(long = "input-trace-path")]
    pub input_trace_path: String,
    /// Whether the passed-in file is a raw trace file or a serialized coverage map
    #[clap(long = "is-raw-trace")]
    pub is_raw_trace_file: bool,
    /// The path to the module binary
    #[clap(long = "module-path")]
    pub module_binary_path: String,
    /// The path to the source file
    #[clap(long = "source-path")]
    pub source_file_path: String,
    /// Optional path to save coverage. Printed to stdout if not present.
    #[clap(long = "coverage-path")]
    pub coverage_path: Option<String>,
    /// Colorize output based on coverage
    #[clap(long, default_value_t = ColorChoice::Default)]
    pub color: ColorChoice,
    /// Tag each line with a textual indication of coverage
    #[clap(long, default_value_t = TextIndicator::Explicit)]
    pub tag: TextIndicator,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let source_map_extension = SOURCE_MAP_EXTENSION;
    let coverage_map = if args.is_raw_trace_file {
        CoverageMap::from_trace_file(&args.input_trace_path)?
    } else {
        CoverageMap::from_binary_file(&args.input_trace_path)?
    };

    let bytecode_bytes =
        fs::read(&args.module_binary_path).with_context(|| "Unable to read bytecode file")?;
    let compiled_module = CompiledModule::deserialize(&bytecode_bytes)
        .with_context(|| "Module blob can't be deserialized")?;

    let source_map = source_map_from_file(
        &Path::new(&args.module_binary_path).with_extension(source_map_extension),
    )?;
    let source_path = Path::new(&args.source_file_path);
    let source_cov = SourceCoverageBuilder::new(&coverage_map, &source_map, vec![(
        &compiled_module,
        &source_map,
    )]);

    let mut coverage_writer: Box<dyn Write> = match &args.coverage_path {
        Some(x) => {
            let path = Path::new(x);
            Box::new(File::create(path).unwrap())
        },
        None => Box::new(io::stdout()),
    };

    let source_coverage = source_cov.compute_source_coverage(source_path);
    source_coverage.output_source_coverage(&mut coverage_writer, args.color, args.tag)?;
    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
