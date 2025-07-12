// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use clap::{Arg, Command};

fn main() {
    let matches = Command::new("Fuzz CLI")
        .version("0.1")
        .author("Security Team @ Aptos Labs")
        .about("This CLI is used to help craft and maintain fuzz targets for the Core components of the Aptos Blockchain.")
        .subcommand(
            Command::new("compile_federated_jwk")
                .about("Compiles a module from source and dumps serialized metadata and code to be used as static initializers in fuzz targets.")
                .arg(
                    Arg::new("module_path")
                        .help("Path to the module source")
                        .required(true)
                        .index(1),
                )
        )
        .subcommand(
            Command::new("generate_runnable_state")
                .about("Generates a runnable state from a Move module and its metadata.")
                .arg(
                    Arg::new("csv_path")
                        .help("Path to a csv containing b64 encode modules in third column")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("destination_path")
                        .help("Path to write the runnable state to")
                        .required(true)
                        .index(2),
                )
        )
        .subcommand(
            Command::new("generate_runnable_state_from_project")
                .about("Generates a runnable state from a project.")
                .arg(
                    Arg::new("project_path")
                        .help("Path to the project to generate the runnable state from (e.g. ../data/transfer-resources)")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("destination_path")
                        .help("Path to write the runnable state to")
                        .required(true)
                        .index(2),
                )
        )
        .subcommand(
            Command::new("generate_runnable_states_recursive")
                .about("Recursively generates runnable states from all Move.toml files in a directory.")
                .arg(
                    Arg::new("base_dir")
                        .help("Base directory to search for Move.toml files")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("destination_path")
                        .help("Path to write the runnable states to")
                        .required(true)
                        .index(2),
                )
        )
        .subcommand(
            Command::new("generate_runnable_states_from_all_tests")
                .about("Generates runnable states from all test sources (e2e, transactional, and compiler v2 tests).")
                .arg(
                    Arg::new("destination_path")
                        .help("Path to write the runnable states to")
                        .required(true)
                        .index(1),
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("compile_federated_jwk", sub_m)) => {
            let module_path = sub_m.get_one::<String>("module_path").unwrap();

            // Call the function with the provided arguments
            if let Err(e) = fuzzer::utils::cli::compile_federated_jwk(module_path) {
                eprintln!("Error compiling module: {}", e);
                std::process::exit(1);
            } else {
                println!("Module compiled successfully.");
            }
        },
        Some(("generate_runnable_state", sub_m)) => {
            let csv_path = sub_m.get_one::<String>("csv_path").unwrap();
            let destination_path = sub_m.get_one::<String>("destination_path").unwrap();

            // Call the function with the provided arguments
            if let Err(e) = fuzzer::utils::cli::generate_runnable_state(csv_path, destination_path)
            {
                eprintln!("Error generating runnable state: {}", e);
                std::process::exit(1);
            } else {
                println!("Runnable state generated successfully.");
            }
        },
        Some(("generate_runnable_state_from_project", sub_m)) => {
            let project_path = sub_m.get_one::<String>("project_path").unwrap();
            let destination_path = sub_m.get_one::<String>("destination_path").unwrap();

            // Call the function with the provided arguments
            if let Err(e) = fuzzer::utils::cli::generate_runnable_state_from_project(
                project_path,
                destination_path,
            ) {
                eprintln!("Error generating runnable state: {}", e);
                std::process::exit(1);
            } else {
                println!("Runnable state generated successfully.");
            }
        },
        Some(("generate_runnable_states_recursive", sub_m)) => {
            let base_dir = sub_m.get_one::<String>("base_dir").unwrap();
            let destination_path = sub_m.get_one::<String>("destination_path").unwrap();

            // Call the function with the provided arguments
            if let Err(e) =
                fuzzer::utils::cli::generate_runnable_states_recursive(base_dir, destination_path)
            {
                eprintln!("Error generating runnable states recursively: {}", e);
                std::process::exit(1);
            } else {
                println!("Runnable states generated successfully.");
            }
        },
        Some(("generate_runnable_states_from_all_tests", sub_m)) => {
            let destination_path = sub_m.get_one::<String>("destination_path").unwrap();

            // Call the function with the provided arguments
            if let Err(e) =
                fuzzer::utils::cli::generate_runnable_states_from_all_tests(destination_path)
            {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            } else {
                println!("Runnable states generated successfully from all test sources.");
            }
        },
        // Handle other subcommands or default behavior
        _ => {
            println!("No valid subcommand was used. Use --help for more information.");
        },
    }
}
