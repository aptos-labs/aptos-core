// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License

//! WASI-compatible CLI for Move compiler
//!
//! This can be run in browsers using a WASI polyfill

use std::io::{self, Read, Write};

fn main() {
    eprintln!("Move Compiler WASI CLI v0.1.0");

    // Read input from stdin (JSON format)
    let mut input = String::new();
    io::stdin().read_to_string(&mut input).expect("Failed to read stdin");

    // Parse input JSON
    let request: serde_json::Value = serde_json::from_str(&input)
        .expect("Invalid JSON input");

    let action = request["action"].as_str().expect("Missing action field");

    match action {
        "compile_module" => {
            let source = request["source"].as_str().expect("Missing source");
            let address = request["address"].as_str().expect("Missing address");
            let module_name = request["module_name"].as_str().expect("Missing module_name");

            // Write temp file
            std::fs::write("temp.move", source).expect("Failed to write temp file");

            // Use the library's internal compile function
            let result = move_compiler_wasm::compile_module(
                source.to_string(),
                address.to_string(),
                module_name.to_string(),
            );

            // Clean up
            let _ = std::fs::remove_file("temp.move");

            // Output result as JSON
            println!("{}", result.to_json());
        }
        _ => {
            eprintln!("Unknown action: {}", action);
            std::process::exit(1);
        }
    }
}
