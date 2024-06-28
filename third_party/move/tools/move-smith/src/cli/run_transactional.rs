// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Simple CLI tool that reads transactional test from a file and runs it.

use move_smith::{config::Config, utils::run_transactional_test};
use std::{fs, path::PathBuf};

fn main() {
    // Read the input file path from the first command line argument
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: run_transactional <input_file>");
        std::process::exit(1);
    }

    let file_path = PathBuf::from(&args[1]);
    let code = fs::read_to_string(&file_path).unwrap();
    println!("Loaded code from file: {:?}", file_path);

    println!("Running the transactional test");
    run_transactional_test(code, &Config::default()).unwrap();
    println!("Transactional test passed");
}
