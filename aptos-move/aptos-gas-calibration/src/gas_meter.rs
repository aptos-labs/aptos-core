// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::gas_meter_helpers::{get_dir_paths, get_functional_identifiers, record_gas_meter};
use aptos_framework::{BuildOptions, BuiltPackage};
use aptos_gas_algebra::Expression;
use aptos_language_e2e_tests::executor::FakeExecutor;
use move_binary_format::CompiledModule;
use move_ir_compiler::Compiler;
use std::{
    fs::{read_dir, read_to_string},
    path::PathBuf,
    time::Instant,
};

pub struct GasMeters {
    pub regular_meter: Vec<u128>,
    pub abstract_meter: Vec<Vec<Expression>>,
}

/// Compile every Move sample and run each sample with two different measuring methods.
/// The first is with the Regular Gas Meter (used in production) to record the running time.
/// The second is with the Abstract Algebra Gas Meter to record abstract gas usage.
pub fn compile_and_run_samples() -> GasMeters {
    // Discover all top-level packages in samples directory
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples");
    let dirs = read_dir(path.as_path()).unwrap();
    let dir_paths = get_dir_paths(dirs);

    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    let mut gas_meter = GasMeters {
        regular_meter: Vec::new(),
        abstract_meter: Vec::new(),
    };

    // Go over all Move projects
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

            // get module address
            let address = module_id.address();

            // get all benchmark tagged functions
            // TODO: change pattern
            let func_identifiers =
                get_functional_identifiers(compiled_module, identifier, *address, String::new());

            let meter_results = record_gas_meter(
                &package,
                &mut executor,
                func_identifiers,
                *address,
                identifier,
            );

            // record with regular gas meter
            gas_meter.regular_meter.extend(meter_results.regular_meter);

            // record with abstract gas meter
            gas_meter
                .abstract_meter
                .extend(meter_results.abstract_meter);
        }
    }
    gas_meter
}

/// Compile every MVIR and run each sample with two different measuring methods.
/// The first is with the Regular Gas Meter (used in production) to record the running time.
/// The second is with the Abstract Algebra Gas Meter to record abstract gas usage.
pub fn compile_and_run_samples_ir() -> GasMeters {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples_ir");
    let dirs = read_dir(path.as_path()).unwrap();

    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    let mut gas_meter = GasMeters {
        regular_meter: Vec::new(),
        abstract_meter: Vec::new(),
    };

    for file in dirs {
        if let Ok(file) = file {
            let file_path = file.path();

            if file_path.is_file() {
                // compile module
                let code = read_to_string(&file_path).expect("should have file contents");
                let module = Compiler::new(vec![])
                    .into_compiled_module(&code)
                    .expect("should compile mvir");

                // get relevant module metadata
                let module_id = module.self_id();
                let identifier = &module_id.name().to_string();
                let address = module_id.address();
                let func_identifiers =
                    get_functional_identifiers(module.clone(), identifier, *address, String::new());

                // build .mv of module
                let mut module_blob: Vec<u8> = vec![];
                module
                    .serialize(&mut module_blob)
                    .expect("should serialize");

                println!("BLOB {:#?}\n", module);

                // publish module
                executor.add_module(&module_id, module_blob);

                for func_identifier in func_identifiers {
                    let start = Instant::now();
                    executor.exec_module(&module_id, &func_identifier, vec![], vec![]);
                    let elapsed = start.elapsed();
                    gas_meter.regular_meter.push(elapsed.as_micros());

                    // record with abstract gas meter
                    let gas_formula =
                        executor.exec_abstract_usage(&module_id, &func_identifier, vec![], vec![]);
                    gas_meter.abstract_meter.push(gas_formula);
                }
            }
        }
    }

    gas_meter
}
