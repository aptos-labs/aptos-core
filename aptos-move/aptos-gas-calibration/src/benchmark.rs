// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::benchmark_helpers;
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_language_e2e_tests::executor::FakeExecutor;
use move_binary_format::CompiledModule;
use std::{fs::read_dir, path::PathBuf};

pub fn benchmark_calibration_function() -> Vec<u128> {
    //// Discover all top-level packages in samples directory
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples");
    let dirs = read_dir(path.as_path()).unwrap();

    //// Setting up local execution environment once
    // disable parallel execution
    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    //// get all paths for Move projects
    let dir_paths = benchmark_helpers::get_dir_paths(dirs);

    //// get all running time of every benchmark function
    let mut module_durations: Vec<u128> = Vec::new();

    //// Go over all Move projects
    for dir_path in dir_paths {
        // configure and build Move package
        let build_options = BuildOptions {
            with_srcs: true,
            with_abis: true,
            with_source_maps: true,
            with_error_map: true,
            ..BuildOptions::default()
        };
        let package =
            BuiltPackage::build(dir_path, build_options).expect("build package must succeed");

        // iterate over all Move package code
        let codes = package.extract_code();
        for code in codes {
            let compiled_module = CompiledModule::deserialize(&code).unwrap();
            let module_id = compiled_module.self_id();
            let identifier = &module_id.name().to_string();

            //// get module address
            let address = module_id.address();

            //// get all benchmark tagged functions
            let func_identifiers = benchmark_helpers::get_functional_identifiers(
                compiled_module,
                identifier,
                *address,
                String::new(), // TODO: pattern matching support
            );

            //// publish module and sign user transaction
            //// the benchmark happens when in signing user txn
            let durations = benchmark_helpers::record_with_regular_gas_meter(
                &package,
                &mut executor,
                func_identifiers,
                *address,
                identifier,
            );

            module_durations.extend(durations);
        }
    }

    module_durations
}
