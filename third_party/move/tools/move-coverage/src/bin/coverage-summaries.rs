// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use anyhow::{bail, Context};
use clap::Parser;
use move_binary_format::file_format::CompiledModule;
use move_coverage::{
    coverage_map::{CoverageMap, TraceMap},
    format_csv_summary, format_human_summary, summary,
};
use std::{
    fs::{self, File},
    io::{self, Write},
    path::Path,
};

#[derive(Debug, Parser)]
#[clap(
    name = "coverage-summaries",
    about = "Creates a coverage summary from the trace data collected from the Move VM",
    author,
    version
)]
struct Args {
    /// The path to the coverage map file
    #[clap(long = "input-trace-path", short = 't')]
    pub input_trace_path: String,
    /// Whether the passed-in file is a raw trace file (true) or a serialized coverage map (false)
    #[clap(long = "is-raw-trace", short = 'r')]
    pub is_raw_trace_file: bool,
    /// The path to the module binary
    #[clap(long = "module-path", short = 'b')]
    /// The path to a directory containing binaries to be processed (e.g., `./build/*/bytecode_modules`)
    pub module_binary_path: Option<String>,
    #[clap(long = "modules-dir", short = 'm')]
    pub modules_dir: Option<String>,
    /// Optional path for summaries. Printed to stdout if not present.
    #[clap(long = "summary-path", short = 'o')]
    pub summary_path: Option<String>,
    /// Whether function coverage summaries should be displayed
    #[clap(long = "summarize-functions", short = 'f')]
    pub summarize_functions: bool,
    /// The path to the standard library binary directory for Move (e.g., `.../build/MoveStdlib/bytecode_modules`)
    #[clap(long = "stdlib-path", short = 's')]
    pub stdlib_path: Option<String>,
    /// Whether path coverage should be derived (default is instruction coverage)
    #[clap(long = "derive-path-coverage", short = 'p')]
    pub derive_path_coverage: bool,
    /// Output CSV data of coverage
    #[clap(long = "csv", short = 'c')]
    pub csv_output: bool,
}

fn maybe_add_modules(
    modules: &mut Vec<CompiledModule>,
    modules_dir: &Option<String>,
) -> anyhow::Result<()> {
    if let Some(modules_path) = modules_dir {
        let new_modules_files = fs::read_dir(modules_path)
            .with_context(|| format!("Reading module directory {}", modules_path.to_string()))?;
        let new_modules: Result<Vec<CompiledModule>, _> = new_modules_files
            .map(|dirent_or_error| {
                dirent_or_error
                    .with_context(|| {
                        format!(
                            "Iterating over module directory {}",
                            modules_path.to_string()
                        )
                    })
                    .and_then(|file| {
                        fs::read(file.path())
                            .map_err(anyhow::Error::from)
                            .and_then(|bytes| {
                                CompiledModule::deserialize(&bytes).with_context(|| {
                                    format!("Reading file {}", file.path().to_string_lossy())
                                })
                            })
                    })
            })
            .collect();
        let mut new_modules = new_modules?;
        modules.append(&mut new_modules);
    }
    Ok(())
}

fn get_modules(args: &Args) -> anyhow::Result<Vec<CompiledModule>> {
    let mut modules = Vec::new();
    maybe_add_modules(&mut modules, &args.stdlib_path)?;
    maybe_add_modules(&mut modules, &args.modules_dir)?;

    if let Some(module_binary_path) = &args.module_binary_path {
        let bytecode_bytes = fs::read(module_binary_path).with_context(|| {
            format!(
                "Failed to get_modules for module path {}",
                module_binary_path
            )
        })?;
        let compiled_module = CompiledModule::deserialize(&bytecode_bytes).with_context(|| {
            format!(
                "Filed to get_modules for module path {}",
                module_binary_path
            )
        })?;
        modules.push(compiled_module);
    }

    if modules.is_empty() {
        bail!("No modules provided for coverage checking")
    }

    Ok(modules)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let input_trace_path = Path::new(&args.input_trace_path);

    let mut summary_writer: Box<dyn Write> = match &args.summary_path {
        Some(x) => {
            let path = Path::new(x);
            Box::new(File::create(path).unwrap())
        },
        None => Box::new(io::stdout()),
    };

    let modules = get_modules(&args)?;
    if args.derive_path_coverage {
        let trace_map = if args.is_raw_trace_file {
            TraceMap::from_trace_file(&input_trace_path).with_context(|| {
                format!("Reading trace file {}", input_trace_path.to_string_lossy())
            })?
        } else {
            TraceMap::from_binary_file(&input_trace_path).with_context(|| {
                format!(
                    "Reading binary coverage file {}",
                    input_trace_path.to_string_lossy()
                )
            })?
        };
        if !args.csv_output {
            format_human_summary(
                &modules,
                &trace_map,
                summary::summarize_path_cov,
                &mut summary_writer,
                args.summarize_functions,
            )
        } else {
            format_csv_summary(
                &modules,
                &trace_map,
                summary::summarize_path_cov,
                &mut summary_writer,
            )
        }
    } else {
        let coverage_map = if args.is_raw_trace_file {
            CoverageMap::from_trace_file(&input_trace_path).with_context(|| {
                format!("Reading trace file {}", input_trace_path.to_string_lossy(),)
            })?
        } else {
            CoverageMap::from_binary_file(&input_trace_path).with_context(|| {
                format!(
                    "Reading binary coverage file {}",
                    input_trace_path.to_string_lossy(),
                )
            })?
        };
        let unified_exec_map = coverage_map.to_unified_exec_map();
        if !args.csv_output {
            format_human_summary(
                &modules,
                &unified_exec_map,
                summary::summarize_inst_cov,
                &mut summary_writer,
                args.summarize_functions,
            )
        } else {
            format_csv_summary(
                &modules,
                &unified_exec_map,
                summary::summarize_inst_cov,
                &mut summary_writer,
            )
        }
    }
    Ok(())
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Args::command().debug_assert()
}
