// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::measurements_helpers::{get_dir_paths, list_entrypoints, record_gas_usage};
use velor_framework::{BuildOptions, BuiltPackage};
use velor_gas_algebra::DynamicExpression;
use velor_language_e2e_tests::executor::{ExecFuncTimerDynamicArgs, FakeExecutor, GasMeterType};
use move_binary_format::CompiledModule;
use move_ir_compiler::Compiler;
use std::{
    fs::{read_dir, read_to_string},
    path::PathBuf,
};
use walkdir::WalkDir;

pub struct GasMeasurements {
    pub regular_meter: Vec<u128>,
    pub abstract_meter: Vec<Vec<DynamicExpression>>,
    pub equation_names: Vec<String>,
}

/// Compile and run both samples and samples_ir directories
pub fn compile_and_run(iterations: u64, pattern: &String) -> GasMeasurements {
    let executor = FakeExecutor::from_head_genesis();
    let mut executor = executor.set_not_parallel();

    let mut gas_measurement = GasMeasurements {
        regular_meter: Vec::new(),
        abstract_meter: Vec::new(),
        equation_names: Vec::new(),
    };

    compile_and_run_samples(iterations, pattern, &mut gas_measurement, &mut executor);
    compile_and_run_samples_ir(iterations, pattern, &mut gas_measurement, &mut executor);

    gas_measurement
}

/// Compile every Move sample and run each sample with two different measuring methods.
/// The first is with the Regular Gas Meter (used in production) to record the running time.
/// The second is with the Abstract Algebra Gas Meter to record abstract gas usage.
fn compile_and_run_samples(
    iterations: u64,
    pattern: &String,
    gas_measurement: &mut GasMeasurements,
    executor: &mut FakeExecutor,
) {
    // Discover all top-level packages in samples directory
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples");
    let dirs = read_dir(path.as_path()).unwrap();
    let dir_paths = get_dir_paths(dirs);

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
            BuiltPackage::build(dir_path, build_options).expect("Failed to build package");

        // iterate over all Move package code
        let codes = package.extract_code();
        for code in codes {
            let compiled_module = CompiledModule::deserialize(&code).unwrap();
            let module_id = compiled_module.self_id();
            let identifier = &module_id.name().to_string();

            // get module address
            let address = module_id.address();

            // get all benchmark tagged functions
            let func_identifiers = list_entrypoints(&compiled_module, pattern.to_string()).expect(
                "Failed: entry function probably has >1 parameter that's not a Signer type",
            );

            let measurement_results = record_gas_usage(
                &package,
                executor,
                func_identifiers,
                *address,
                identifier,
                iterations,
            );

            // record the equation names
            gas_measurement
                .equation_names
                .extend(measurement_results.equation_names);

            // record with regular gas meter
            gas_measurement
                .regular_meter
                .extend(measurement_results.regular_meter);

            // record with abstract gas meter
            gas_measurement
                .abstract_meter
                .extend(measurement_results.abstract_meter);
        }
    }
}

/// Compile every MVIR and run each sample with two different measuring methods.
/// The first is with the Regular Gas Meter (used in production) to record the running time.
/// The second is with the Abstract Algebra Gas Meter to record abstract gas usage.
fn compile_and_run_samples_ir(
    iterations: u64,
    pattern: &String,
    gas_measurement: &mut GasMeasurements,
    executor: &mut FakeExecutor,
) {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("samples_ir");

    // Walk through all subdirectories and files in the root directory
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let path_entry = entry.path();

        // Check if the entry is a file
        if path_entry.is_file() {
            // ignore all non .mvir files
            if let Some(file_ext) = path_entry.extension() {
                if let Some(ext) = file_ext.to_str() {
                    if ext != "mvir" {
                        continue;
                    }
                }
            }

            if let Some(file_name) = path_entry.file_name() {
                // Convert the file_name to a string slice
                if let Some(_file_name_str) = file_name.to_str() {
                    // compile module
                    let code = read_to_string(path_entry).expect("Failed to read file contents");
                    let module = Compiler::new(vec![])
                        .into_compiled_module(&code)
                        .expect("should compile mvir");

                    // get relevant module metadata
                    let module_id = module.self_id();
                    let identifier = &module_id.name().to_string();
                    let func_identifiers = list_entrypoints(&module, pattern.to_string()).expect(
                        "Failed: entry function probably has >1 parameter that's not a Signer type",
                    );

                    // build .mv of module
                    let mut module_blob: Vec<u8> = vec![];
                    module
                        .serialize(&mut module_blob)
                        .expect("Failed to serialize module");

                    // publish module
                    executor.add_module(&module_id, module_blob);

                    for func_identifier in func_identifiers {
                        println!("Benchmarking {}::{}\n", &identifier, func_identifier.0);

                        gas_measurement
                            .equation_names
                            .push(format!("{}::{}", &identifier, func_identifier.0));

                        let elapsed = executor
                            .exec_func_record_running_time(
                                &module_id,
                                &func_identifier.0,
                                vec![],
                                func_identifier.1.clone(),
                                iterations,
                                ExecFuncTimerDynamicArgs::NoArgs,
                                GasMeterType::UnmeteredGasMeter,
                            )
                            .elapsed_micros();
                        gas_measurement.regular_meter.push(elapsed);

                        // record with abstract gas meter
                        let gas_formula = executor.exec_abstract_usage(
                            &module_id,
                            &func_identifier.0,
                            vec![],
                            func_identifier.1,
                        );
                        gas_measurement.abstract_meter.push(gas_formula);
                    }
                }
            }
        }
    }
}
