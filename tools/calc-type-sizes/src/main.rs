// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This tool scans for `.mv` files (compiled Move modules) and performs two analyses:
//!
//! 1. Type Analysis (default): Computes stack sizes and nesting depths for each type.
//! 2. Stack Analysis (--analyze-stack): Traces call graph from entry functions to compute
//!    frame sizes and maximum stack depths.
//!
//! Stack size calculation:
//!   - Primitives: u8 -> 1, u16 -> 2, u32 -> 4, u64 -> 8, u128 -> 16, u256 -> 32
//!   - Signed integers: same sizes as unsigned counterparts
//!   - bool -> 1, address -> 32, signer -> 32
//!   - Vector -> 24 (data is on heap)
//!   - References -> 8 (pointer)
//!   - Function values -> 8 (pointer to boxed object)
//!   - Structs -> sum of field stack sizes
//!   - Enums -> 1 (tag) + max(variant field sizes)

mod histogram;
mod module_loader;
mod resolver;
mod stack_analyzer;
mod types;

use anyhow::Result;
use clap::Parser;
use histogram::Histogram;
use module_loader::{deserialize_modules, read_module_bytes};
use resolver::TypeResolver;
use stack_analyzer::StackAnalyzer;
use std::fmt::Write;
use tokio::fs;
use types::TypeInfo;

#[derive(Parser, Debug)]
#[clap(
    author,
    version,
    about = "Compute stack sizes and call depths for Move types and functions"
)]
struct Args {
    /// Path to search for .mv files (defaults to current directory)
    #[clap(long, value_parser, default_value = ".")]
    path: String,

    /// Output file path (defaults to types.csv or stack.csv depending on mode)
    #[clap(long, value_parser)]
    output: Option<String>,

    /// Stack size to use for opaque (uninstantiated) type parameters
    #[clap(long, value_parser, default_value = "0")]
    opaque_size: usize,

    /// Run stack analysis instead of type analysis
    #[clap(long)]
    analyze_stack: bool,

    /// Generate histogram of type size distribution (type analysis mode only)
    #[clap(long)]
    histogram: bool,

    /// Generate histogram of nesting depth distribution (type analysis mode only)
    #[clap(long)]
    depth_histogram: bool,

    /// Output histogram as CSV instead of ASCII art
    #[clap(long)]
    histogram_csv: bool,

    /// Width of ASCII histogram bars (default 50)
    #[clap(long, value_parser, default_value = "50")]
    histogram_width: usize,
}

/// Render type analysis results as CSV
fn render_types_csv(results: &[(String, &TypeInfo)]) -> Result<String> {
    let mut s = String::new();
    writeln!(s, "type_name,stack_size,nested_depth,kind")?;

    for (name, info) in results {
        writeln!(
            s,
            "\"{}\",{},{},{}",
            name, info.stack_size, info.nested_depth, info.kind
        )?;
    }

    Ok(s)
}

/// Render stack analysis results as CSV
fn render_stack_csv(
    results: &[(
        &stack_analyzer::FunctionId,
        &stack_analyzer::FunctionStackInfo,
    )],
) -> Result<String> {
    let mut s = String::new();
    writeln!(
        s,
        "function,frame_size,max_call_depth,max_stack_size,is_entry,is_recursive"
    )?;

    for (id, info) in results {
        let depth_str = info
            .max_call_depth
            .map(|d| d.to_string())
            .unwrap_or_else(|| "∞".to_string());
        let stack_str = info
            .max_stack_size
            .map(|s| s.to_string())
            .unwrap_or_else(|| "∞".to_string());

        writeln!(
            s,
            "\"{}\",{},{},{},{},{}",
            id, info.frame_size, depth_str, stack_str, info.is_entry, info.is_recursive
        )?;
    }

    Ok(s)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    println!("Scanning for .mv files in: {}", args.path);
    let module_bytes = read_module_bytes(&args.path).await?;
    println!("Found {} .mv files", module_bytes.len());

    let modules = deserialize_modules(&module_bytes);
    println!(
        "Successfully deserialized {} out of {} modules",
        modules.len(),
        module_bytes.len()
    );

    if args.analyze_stack {
        // Stack analysis mode - uses TypeResolver for type computations
        let mut resolver = TypeResolver::with_opaque_size(&modules, args.opaque_size);
        let mut analyzer = StackAnalyzer::new(&modules, &mut resolver);
        analyzer.analyze_all();

        let results = analyzer.get_results();
        println!("Analyzed {} functions", analyzer.function_count());

        let csv = render_stack_csv(&results)?;
        let output = args.output.unwrap_or_else(|| "stack.csv".to_string());
        fs::write(&output, csv).await?;
        println!("Results written to: {}", output);
    } else {
        // Type analysis mode (default)
        let mut resolver = TypeResolver::with_opaque_size(&modules, args.opaque_size);
        resolver.process_all_modules();

        let results = resolver.get_results();
        println!("Computed info for {} types", results.len());

        // Generate size histogram if requested
        if args.histogram || args.histogram_csv {
            let hist = Histogram::from_type_results(&results);

            if args.histogram_csv {
                let csv = hist.render_csv();
                let hist_output = args
                    .output
                    .clone()
                    .map(|o| o.replace(".csv", "-size-histogram.csv"))
                    .unwrap_or_else(|| "types-size-histogram.csv".to_string());
                fs::write(&hist_output, csv).await?;
                println!("Size histogram written to: {}", hist_output);
            } else {
                // ASCII histogram to file
                let ascii = hist.render_ascii(args.histogram_width);
                let hist_output = args
                    .output
                    .clone()
                    .map(|o| o.replace(".csv", "-size-histogram.txt"))
                    .unwrap_or_else(|| "types-size-histogram.txt".to_string());
                fs::write(&hist_output, &ascii).await?;
                println!("Size histogram written to: {}", hist_output);
                println!("\n{}", ascii);
            }
        }

        // Generate depth histogram if requested
        if args.depth_histogram {
            let hist = Histogram::depth_from_type_results(&results);

            if args.histogram_csv {
                let csv = hist.render_csv();
                let hist_output = args
                    .output
                    .clone()
                    .map(|o| o.replace(".csv", "-depth-histogram.csv"))
                    .unwrap_or_else(|| "types-depth-histogram.csv".to_string());
                fs::write(&hist_output, csv).await?;
                println!("Depth histogram written to: {}", hist_output);
            } else {
                // ASCII histogram to file
                let ascii = hist.render_ascii(args.histogram_width);
                let hist_output = args
                    .output
                    .clone()
                    .map(|o| o.replace(".csv", "-depth-histogram.txt"))
                    .unwrap_or_else(|| "types-depth-histogram.txt".to_string());
                fs::write(&hist_output, &ascii).await?;
                println!("Depth histogram written to: {}", hist_output);
                println!("\n{}", ascii);
            }
        }

        let csv = render_types_csv(&results)?;
        let output = args.output.unwrap_or_else(|| "types.csv".to_string());
        fs::write(&output, csv).await?;
        println!("Results written to: {}", output);
    }

    Ok(())
}
