// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Simple CLI tool that statically generates Move source files or packages
//! with random content controlled by a seed.

use clap::Parser;
use move_smith::{utils::raw_to_compile_unit, CodeGenerator};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::{fs, path::PathBuf};

#[derive(Debug, Parser)]
#[clap(author, version, about)]
struct Args {
    /// The output directory to store the generated Move files
    #[clap(short, long)]
    output_dir: PathBuf,

    /// An optional number as seed, the default should be 0
    #[clap(short, long, default_value = "0")]
    seed: u64,

    /// An optional number as the number of files to generate, the default should be 100
    #[clap(short, long, default_value = "100")]
    num_files: usize,

    /// A boolean flag to create a package, default to false
    #[clap(short, long)]
    package: bool,
}

const BUFFER_SIZE_START: usize = 1024 * 4;
const MOVE_TOML_TEMPLATE: &str = r#"[package]
name = "test"
version = "0.0.0"
"#;

fn main() {
    env_logger::init();
    let args = Args::parse();
    fs::create_dir_all(&args.output_dir).expect("Failed to create output directory");

    println!("Using seed: {}", args.seed);
    let mut rng = StdRng::seed_from_u64(args.seed);

    for i in 0..args.num_files {
        println!("MoveSmith: generating file #{}", i);
        let mut buffer_size = BUFFER_SIZE_START;
        let mut buffer = vec![];
        let module = loop {
            if buffer_size > buffer.len() {
                let diff = buffer_size - buffer.len();
                let mut new_buffer = vec![0u8; diff];
                rng.fill(&mut new_buffer[..]);
                buffer.extend(new_buffer);
            }

            match raw_to_compile_unit(&buffer) {
                Ok(module) => break module,
                Err(e) => {
                    if buffer_size > BUFFER_SIZE_START * 1024 {
                        panic!("Failed to parse raw bytes: {}", e);
                    }
                },
            }
            buffer_size *= 2;

            println!("Doubling buffer size to {} bytes", buffer_size);
        };
        println!("Generated MoveSmith instance with {} bytes", buffer_size);

        let code = module.emit_code();
        let file_name = format!("Output_{}.move", i);
        let file_path = match args.package {
            true => {
                let package_dir = args.output_dir.join(format!("Package_{}", i));
                let source_dir = package_dir.join("sources");
                fs::create_dir_all(&source_dir).expect("Failed to create package directory");

                let move_toml_path = package_dir.join("Move.toml");
                fs::write(&move_toml_path, MOVE_TOML_TEMPLATE).expect("Failed to write Move.toml");
                // Write the Move source code
                source_dir.join(file_name)
            },
            false => args.output_dir.join(file_name),
        };
        fs::write(&file_path, code).expect("Failed to write file");
    }

    let output_format = if args.package { "packages" } else { "files" };
    println!(
        "Generated {} {} in {:?}",
        args.num_files, output_format, args.output_dir
    );
}
