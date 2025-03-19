// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use clap::Parser;
use move_coverage::coverage_map::{output_map_to_file, CoverageMap, TraceMap};
use std::path::Path;

#[derive(Debug, Parser)]
#[clap(
    name = "move-trace-conversion",
    about = "Creates a coverage map from the raw data collected from the Move VM",
    author,
    version
)]
struct Args {
    /// The path to the input file
    #[clap(long = "input-file-path")]
    pub input_file_path: String,
    /// The path to the output file location
    #[clap(long = "output-file-path")]
    pub output_file_path: String,
    /// Add traces from `input_file_path` to an existing coverage map at `update_coverage_map`
    #[clap(long = "update")]
    pub update: Option<String>,
    /// Collect structured trace instead of aggregated coverage information
    #[clap(long = "use-trace-map")]
    pub use_trace_map: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let input_path = Path::new(&args.input_file_path);
    let output_path = Path::new(&args.output_file_path);

    if !args.use_trace_map {
        let coverage_map = if let Some(old_coverage_path) = &args.update {
            let path = Path::new(&old_coverage_path);
            let old_coverage_map = CoverageMap::from_binary_file(&path)?;
            old_coverage_map.update_coverage_from_trace_file(&input_path)
        } else {
            CoverageMap::from_trace_file(&input_path)
        };
        let coverage_map = coverage_map?;

        output_map_to_file(&output_path, &coverage_map)?;
    } else {
        let trace_map = if let Some(old_trace_path) = &args.update {
            let path = Path::new(&old_trace_path);
            let old_trace_map = TraceMap::from_binary_file(&path)?;
            old_trace_map.update_from_trace_file(&input_path)
        } else {
            TraceMap::from_trace_file(&input_path)
        };
        let trace_map = trace_map?;

        output_map_to_file(&output_path, &trace_map)?;
    }
    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
