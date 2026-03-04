// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Prints the total number of bytecode instructions in a compiled Move module (.mv file).

use move_binary_format::CompiledModule;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: move-bytecode-stats <file.mv>");
        std::process::exit(1);
    });
    let bytes = std::fs::read(&path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", path, e);
        std::process::exit(1);
    });
    let module = CompiledModule::deserialize(&bytes).unwrap_or_else(|e| {
        eprintln!("Error deserializing {}: {}", path, e);
        std::process::exit(1);
    });
    let total: usize = module
        .function_defs
        .iter()
        .filter_map(|f| f.code.as_ref())
        .map(|c| c.code.len())
        .sum();
    println!("{}", total);
}
