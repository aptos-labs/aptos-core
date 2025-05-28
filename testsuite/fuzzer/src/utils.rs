// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[allow(dead_code)]
#[allow(unused_variables)]
pub mod cli {
    // Import from crate root (lib.rs)
    use crate::{
        Authenticator, ExecVariant, FundAmount, RunnableState, RunnableStateWithOperations,
        UserAccount,
    };
    use aptos_framework::{BuildOptions, BuiltPackage};
    use aptos_types::{
        account_address::AccountAddress,
        transaction::{EntryFunction, Script, TransactionPayload},
    };
    // Import traits for their methods
    use arbitrary::Arbitrary;
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use dearbitrary::Dearbitrary;
    use move_binary_format::{
        access::ModuleAccess,
        file_format::{CompiledModule, CompiledScript, FunctionDefinitionIndex, SignatureToken},
    };
    use move_core_types::{ident_str, identifier::Identifier, language_storage::ModuleId};
    use move_model::metadata::{CompilerVersion, LanguageVersion};
    // Import official transactional test runner functions
    use move_transactional_test_runner::{
        tasks::{taskify, EmptyCommand, TaskCommand, TaskInput},
        vm_test_harness::{
            precompiled_v2_stdlib_fuzzer, AdapterExecuteArgs, AdapterPublishArgs,
            PrecompiledFilesModules,
        },
    };
    use rayon::prelude::*;
    use sha2::{Digest, Sha256};
    use std::{
        collections::{HashMap, VecDeque},
        fs::File,
        io::Write,
        path::PathBuf,
    };
    use walkdir::WalkDir;

    macro_rules! debug_println_error {
        ($($arg:tt)*) => {
            if std::env::var("DEBUG").unwrap_or_default() == "1" {
                println!($($arg)*);
            }
        };
    }

    /// Standard build options for consistent compilation across the fuzzer
    pub(crate) fn standard_build_options() -> BuildOptions {
        BuildOptions {
            language_version: Some(LanguageVersion::default()), // This is V2_1 (latest stable)
            compiler_version: Some(CompilerVersion::latest_stable()), // This is V2_0
            bytecode_version: Some(8), // Use VERSION_MAX to match official transactional test runner
            ..BuildOptions::default()
        }
    }

    /// Compiles a Move module from source code.
    /// The compiled module and its metadata are returned serialized.
    /// Those can be used to publish the module on-chain via code_publish_package_txn().
    pub fn compile_federated_jwk(module_path: &str) -> Result<(), String> {
        let package = BuiltPackage::build(PathBuf::from(module_path), standard_build_options())
            .map_err(|e| e.to_string())?;

        let transaction_payload = generate_script_payload_jwk(&package);
        let code_snippet = format!(
            r#"
            let tx = acc
                .transaction()
                .gas_unit_price(100)
                .sequence_number(sequence_number)
                .payload(bcs::from_bytes(&{:?}).unwrap())
                .sign();
            "#,
            bcs::to_bytes(&transaction_payload).unwrap()
        );
        println!("{}", code_snippet);

        Ok(())
    }

    /// Generate a TransactionPayload for modules
    ///
    /// ### Arguments
    ///
    /// * `package` - Built Move package
    fn generate_module_payload(package: &BuiltPackage) -> TransactionPayload {
        // extract package data
        let code = package.extract_code();
        let metadata = package
            .extract_metadata()
            .expect("extracting package metadata must succeed");

        // publish package similar to create_publish_package in harness.rs
        code_publish_package_txn(
            bcs::to_bytes(&metadata).expect("PackageMetadata has BCS"),
            code,
        )
    }

    /// Generate a TransactionPayload for scripts
    ///
    /// ### Arguments
    ///
    /// * `package` - Built Move package
    fn generate_script_payload_jwk(package: &BuiltPackage) -> TransactionPayload {
        // extract package data
        let code = package.extract_script_code().into_iter().next().unwrap();
        let ty_args = vec![];
        let args = vec![];

        TransactionPayload::Script(Script::new(code, ty_args, args))
    }

    /// Same as `publish_package` but as an entry function which can be called as a transaction. Because
    /// of current restrictions for txn parameters, the metadata needs to be passed in serialized form.
    pub fn code_publish_package_txn(
        metadata_serialized: Vec<u8>,
        code: Vec<Vec<u8>>,
    ) -> TransactionPayload {
        TransactionPayload::EntryFunction(EntryFunction::new(
            ModuleId::new(
                AccountAddress::new([
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 1,
                ]),
                ident_str!("code").to_owned(),
            ),
            ident_str!("publish_package_txn").to_owned(),
            vec![],
            vec![
                bcs::to_bytes(&metadata_serialized).unwrap(),
                bcs::to_bytes(&code).unwrap(),
            ],
        ))
    }

    /// Parse a transactional test file
    ///
    /// This uses the framework taskify function to parse transactional test files
    pub fn parse_transactional_test(
        file_path: &std::path::Path,
    ) -> Result<
        VecDeque<
            TaskInput<
                TaskCommand<EmptyCommand, AdapterPublishArgs, (), AdapterExecuteArgs, EmptyCommand>,
            >,
        >,
        String,
    > {
        taskify(file_path)
            .map(|vec| vec.into_iter().collect())
            .map_err(|e| e.to_string())
    }

    /// Convert transactional test to runnable state using the framework's compilation logic
    pub fn transactional_test_to_runnable_state(
        file_path: &std::path::Path,
        pre_compiled_deps: &PrecompiledFilesModules,
    ) -> Result<RunnableStateWithOperations, String> {
        use move_model::metadata::LanguageVersion;
        use move_transactional_test_runner::{
            framework::MoveTestAdapter,
            tasks::SyntaxChoice,
            transactional_ops::{tasks_to_transactional_operations, MinimalAdapter},
            vm_test_harness::TestRunConfig,
        };

        // Parse the transactional test file
        let mut tasks = parse_transactional_test(file_path)?;

        // Create a minimal test configuration
        let run_config = TestRunConfig::compiler_v2(LanguageVersion::latest(), vec![]);

        let first_task = tasks.pop_front().unwrap();
        let init_opt = match &first_task.command {
            TaskCommand::Init(_, _) => Some(first_task.map(|known| match known {
                TaskCommand::Init(command, extra_args) => (command, extra_args),
                _ => unreachable!(),
            })),
            _ => {
                tasks.push_front(first_task);
                None
            },
        };

        // Create our minimal adapter
        let (mut adapter, _init_output) = MinimalAdapter::init(
            SyntaxChoice::Source,
            run_config,
            pre_compiled_deps,
            init_opt,
        );

        // Use the framework's helper function to convert tasks to operations
        let operations = tasks_to_transactional_operations(&mut adapter, tasks)
            .map_err(|e| format!("Failed to convert tasks to operations: {}", e))?;

        if operations.is_empty() {
            return Err("No valid operations found in transactional test".to_string());
        }

        // Limit the number of operations to prevent performance issues
        const MAX_OPERATIONS: usize = 10;
        let operations = if operations.len() > MAX_OPERATIONS {
            operations.into_iter().take(MAX_OPERATIONS).collect()
        } else {
            operations
        };

        Ok(RunnableStateWithOperations {
            operations,
            tx_auth_type: Authenticator::Ed25519 {
                _sender: UserAccount {
                    is_inited_and_funded: true,
                    fund: FundAmount::Rich,
                },
            },
        })
    }

    /// Convert module to RunnableState data structure
    /// This is used to run the module in the VM
    pub fn generate_runnable_state(csv_path: &str, destination_path: &str) -> Result<(), String> {
        std::fs::create_dir_all(destination_path).map_err(|e| e.to_string())?;
        let runnable_states = to_runnablestate_from_csv(&read_csv(csv_path));
        let mut cnt: usize = 0;
        println!("Number of runnable states: {} \n", runnable_states.len());
        for runnable_state in runnable_states {
            let d = runnable_state.0.dearbitrary_first();
            let bytes = d.finish();

            let u = arbitrary::Unstructured::new(&bytes);
            let runnable_state_generated =
                RunnableState::arbitrary_take_rest(u).map_err(|e| e.to_string())?;
            if runnable_state.0 != runnable_state_generated {
                debug_println_error!(
                    "Failed <CompiledModule>: {:#?}",
                    runnable_state.0.dep_modules[0]
                );
                debug_println_error!("Failed <Serialized>: {:#?} \n", runnable_state.1);
                cnt += 1;
            }
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let hash = hasher.finalize();
            let filename = format!("{}/{}.bytes", destination_path, hex::encode(hash));
            let mut file = File::create(&filename).map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
        }
        println!("Number of different states: {}", cnt);

        Ok(())
    }

    // function that read a CSV files and get the third element of each row and return a vector of bytes
    fn read_csv(path: &str) -> HashMap<ModuleId, (CompiledModule, String)> {
        let mut reader = csv::Reader::from_path(path).unwrap();

        let mut v = HashMap::new();

        for result in reader.records() {
            let record = result.unwrap();

            let module_name = record.get(0).unwrap();
            let address = record.get(1).unwrap();
            let bytecode = match STANDARD.decode(record.get(2).unwrap()) {
                Ok(bytecode) => bytecode,
                Err(err) => panic!(
                    "Error decoding {:?} B64 Decoding error: {:?}, Base64 string: {:?}",
                    address,
                    err,
                    record.get(2).unwrap()
                ),
            };
            let serialized = record.get(2).unwrap();

            let account_address = match AccountAddress::from_hex_literal(address) {
                Ok(addr) => addr,
                Err(err) => {
                    debug_println_error!("Invalid address {}: {:?}", address, err);
                    continue;
                },
            };
            let identifier = match Identifier::new(module_name) {
                Ok(id) => id,
                Err(err) => {
                    debug_println_error!("Invalid module name {}: {:?}", module_name, err);
                    continue;
                },
            };
            let key = ModuleId::new(account_address, identifier);
            let compiled_module = match CompiledModule::deserialize(&bytecode) {
                Ok(module) => module,
                Err(err) => {
                    debug_println_error!("Error deserializing module: {:?}", err);
                    continue; // Skip to the next iteration of the loop
                },
            };
            v.insert(key, (compiled_module, serialized.to_string()));
        }
        v
    }

    fn to_runnablestate_from_csv(
        map: &HashMap<ModuleId, (CompiledModule, String)>,
    ) -> Vec<(RunnableState, String)> {
        map.iter()
            .map(|(module_id, tuple)| {
                let runnable_state = RunnableState {
                    dep_modules: vec![tuple.0.to_owned()],
                    exec_variant: ExecVariant::CallFunction {
                        _module: module_id.to_owned(),
                        _function: FunctionDefinitionIndex::new(0),
                        _type_args: vec![],
                        _args: vec![],
                    },
                    tx_auth_type: Authenticator::Ed25519 {
                        _sender: UserAccount {
                            is_inited_and_funded: true,
                            fund: FundAmount::Rich,
                        },
                    },
                };
                (runnable_state, tuple.1.to_owned())
            })
            .collect()
    }

    // Helper function to check if a signature token is a signer or signer reference
    fn is_signer_or_reference(token: &SignatureToken) -> bool {
        match token {
            SignatureToken::Signer => true,
            SignatureToken::Reference(inner) => matches!(**inner, SignatureToken::Signer),
            _ => false,
        }
    }

    fn to_runnablestate_from_script(
        script: Vec<u8>,
        all_modules: &[CompiledModule],
    ) -> Result<RunnableState, String> {
        let compiled_script = CompiledScript::deserialize(&script)
            .map_err(|e| format!("Failed to deserialize script: {}", e))?;

        Ok(RunnableState {
            dep_modules: all_modules.to_vec(), // TODO: Check which module in all_modules is actually a dependency
            exec_variant: ExecVariant::Script {
                _script: compiled_script,
                _type_args: vec![], // Default to no type args
                _args: vec![],      // Default to no args
            },
            tx_auth_type: Authenticator::Ed25519 {
                _sender: UserAccount {
                    is_inited_and_funded: true,
                    fund: FundAmount::Rich,
                },
            },
        })
    }

    // TODO: Refactor to build arguments for the function
    // TODO2: Check which module in all_modules is actually a dependency
    fn to_runnablestate_from_module(
        module: CompiledModule,
        all_modules: &[CompiledModule],
    ) -> Result<Vec<RunnableState>, String> {
        let mut runnable_states = Vec::new();

        // Find all entry functions in the module
        for (func_idx, func_def) in module.function_defs.iter().enumerate() {
            if func_def.is_entry {
                let module_id = module.self_id();
                let function_handle = &module.function_handles[func_def.function.0 as usize];

                // Skip generic functions
                if !function_handle.type_parameters.is_empty() {
                    debug_println_error!(
                        "Skipping generic entry function {}::{}",
                        module_id,
                        module.identifier_at(function_handle.name)
                    );
                    continue;
                }

                let function_signature = &module.signatures[function_handle.parameters.0 as usize];

                // Skip functions with non-signer parameters after signer parameters
                if !function_signature.0.is_empty()
                    && !is_signer_or_reference(&function_signature.0[0])
                {
                    debug_println_error!(
                        "Skipping function {}::{} with non-signer parameters after signer",
                        module_id,
                        module.identifier_at(function_handle.name)
                    );
                    continue;
                }

                let runnable_state = RunnableState {
                    dep_modules: all_modules.to_vec(),
                    exec_variant: ExecVariant::CallFunction {
                        _module: module_id,
                        _function: FunctionDefinitionIndex::new(func_idx as u16),
                        _type_args: vec![],
                        _args: vec![],
                    },
                    tx_auth_type: Authenticator::Ed25519 {
                        _sender: UserAccount {
                            is_inited_and_funded: true,
                            fund: FundAmount::Rich,
                        },
                    },
                };

                runnable_states.push(runnable_state);
            }
        }

        Ok(runnable_states)
    }

    fn to_runnablestate_from_package(package: &BuiltPackage) -> Result<Vec<RunnableState>, String> {
        let mut runnable_states = Vec::new();
        let modules = package.extract_code();
        let scripts = package.extract_script_code();

        // Process all modules
        let mut compiled_modules = Vec::new();
        for module_bytes in modules {
            if let Ok(module) = CompiledModule::deserialize(&module_bytes) {
                compiled_modules.push(module);
            }
        }

        // Process each module
        for module in &compiled_modules {
            match to_runnablestate_from_module(module.clone(), &compiled_modules) {
                Ok(states) => runnable_states.extend(states),
                Err(e) => {
                    debug_println_error!("Failed to process module {}: {}", module.self_id(), e)
                },
            }
        }

        // Process all scripts
        for script in scripts {
            match to_runnablestate_from_script(script, &compiled_modules) {
                Ok(state) => runnable_states.push(state),
                Err(e) => debug_println_error!("Failed to process script: {}", e),
            }
        }

        if runnable_states.is_empty() {
            return Err("No valid runnable states found in package".to_string());
        }

        Ok(runnable_states)
    }

    fn compile_source_code_from_project(project_path: &str) -> Result<BuiltPackage, String> {
        // Wrap the build in catch_unwind to handle panics
        match std::panic::catch_unwind(|| {
            BuiltPackage::build(
                PathBuf::from(project_path),
                BuildOptions::move_2().set_latest_language(),
            )
        }) {
            Ok(result) => result.map_err(|e| e.to_string()),
            Err(e) => {
                let error_msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic during build".to_string()
                };
                Err(format!("Build panicked: {}", error_msg))
            },
        }
    }

    /// Given a path to a project directory and a destination path,
    /// this function compiles the project, converts it into RunnableState(s),
    /// and writes these states to individual files in the destination directory.
    pub fn generate_runnable_state_from_project(
        project_path: &str,
        destination_path: &str,
    ) -> Result<(), String> {
        std::fs::create_dir_all(destination_path).map_err(|e| e.to_string())?;

        let package = compile_source_code_from_project(project_path)?;
        let runnable_states = to_runnablestate_from_package(&package)?;

        for (idx, runnable_state) in runnable_states.into_iter().enumerate() {
            // Serialize the runnable state
            let bytes = runnable_state.dearbitrary_first().finish();
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let hash = hasher.finalize();

            // Generate filename based on the type of runnable state
            let filename = match &runnable_state.exec_variant {
                ExecVariant::Script { .. } => {
                    format!("{}/script_{}.bytes", destination_path, hex::encode(hash))
                },
                ExecVariant::CallFunction {
                    _module: module, ..
                } => {
                    let mut hasher = Sha256::new();
                    hasher.update(&bytes);
                    format!(
                        "{}/module_{}_{}.bytes",
                        destination_path,
                        module,
                        hex::encode(hasher.finalize())
                    )
                },
            };

            // Write to file
            if let Err(e) = File::create(&filename).and_then(|mut f| f.write_all(&bytes)) {
                debug_println_error!("Failed to write runnable state {}: {}", idx, e);
            } else {
                println!("Runnable state saved to {}", filename);
            }
        }

        Ok(())
    }

    /// Recursively finds all Move.toml files within a given base directory,
    /// compiles each corresponding project, converts them into RunnableState(s),
    /// and writes these states to individual files in a specified destination directory.
    /// Skips any projects that fail to compile or convert.
    pub fn generate_runnable_states_recursive(
        base_dir: &str,
        destination_path: &str,
    ) -> Result<(), String> {
        std::fs::create_dir_all(destination_path).map_err(|e| e.to_string())?;

        // First collect all Move.toml files
        let toml_files: Vec<_> = WalkDir::new(base_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|entry| entry.file_name().to_str() == Some("Move.toml"))
            .map(|entry| entry.path().parent().unwrap().to_string_lossy().to_string())
            .collect();

        println!("Found {} Move projects", toml_files.len());

        // Process projects in parallel using rayon
        toml_files.par_iter().for_each(|project_dir| {
            println!("Processing Move project at: {}", project_dir);
            // Process this Move project
            match compile_source_code_from_project(project_dir) {
                Ok(package) => {
                    match to_runnablestate_from_package(&package) {
                        Ok(runnable_states) => {
                            println!("Generated runnable states from package at {}", project_dir);

                            // Serialize the runnable states
                            for (idx, runnable_state) in runnable_states.into_iter().enumerate() {
                                let bytes = runnable_state.dearbitrary_first().finish();

                                // Generate filename based on the type of runnable state
                                let filename = match &runnable_state.exec_variant {
                                    ExecVariant::Script { .. } => {
                                        format!("{}/script_{}.bytes", destination_path, idx)
                                    },
                                    ExecVariant::CallFunction {
                                        _module: module, ..
                                    } => {
                                        let mut hasher = Sha256::new();
                                        hasher.update(&bytes);
                                        format!(
                                            "{}/module_{}_{}.bytes",
                                            destination_path,
                                            module,
                                            hex::encode(hasher.finalize())
                                        )
                                    },
                                };

                                // Write to file
                                if let Err(e) =
                                    File::create(&filename).and_then(|mut f| f.write_all(&bytes))
                                {
                                    debug_println_error!(
                                        "Failed to write runnable state {}: {}",
                                        idx,
                                        e
                                    );
                                } else {
                                    println!("Runnable state saved to {}", filename);
                                }
                            }
                        },
                        Err(e) => {
                            debug_println_error!(
                                "Failed to generate runnable states for {}: {}",
                                project_dir,
                                e
                            );
                        },
                    }
                },
                Err(e) => {
                    debug_println_error!("Failed to compile project at {}: {}", project_dir, e);
                },
            }
        });

        Ok(())
    }

    /// Generates runnable states from all known test sources, including
    /// e2e tests, transactional tests, and compiler v2 tests.
    /// The generated states are written to individual files in the specified destination directory.
    /// This function leverages precompiled standard library modules for efficiency.
    pub fn generate_runnable_states_from_all_tests(destination_path: &str) -> Result<(), String> {
        std::fs::create_dir_all(destination_path).map_err(|e| e.to_string())?;

        // Build once the framework
        let pre_compiled_deps = precompiled_v2_stdlib_fuzzer();

        // Process transactional tests from move-compiler-v2
        let transactional_test_dirs = vec![
            "",
            "../../third_party/move/move-compiler-v2/transactional-tests/tests",
        ];

        for test_dir in transactional_test_dirs {
            if std::path::Path::new(test_dir).exists() {
                println!("Processing transactional tests from: {}", test_dir);

                for entry in WalkDir::new(test_dir)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "move"))
                {
                    match transactional_test_to_runnable_state(entry.path(), pre_compiled_deps) {
                        Ok(runnable_state) => {
                            if runnable_state.operations.is_empty() {
                                debug_println_error!("Skipping empty transactional test (no operations produced): {}", entry.path().display());
                                continue;
                            }

                            let bytes = runnable_state.dearbitrary_first().finish();
                            let mut hasher = Sha256::new();
                            hasher.update(&bytes);
                            let hash = hasher.finalize();

                            let filename = format!(
                                "{}/transactional_test_{}.bytes",
                                destination_path,
                                hex::encode(hash)
                            );

                            if let Err(e) =
                                File::create(&filename).and_then(|mut f| f.write_all(&bytes))
                            {
                                debug_println_error!(
                                    "Failed to write transactional test {}: {}",
                                    entry.path().display(),
                                    e
                                );
                            } else {
                                println!("Transactional test saved to {}", filename);
                            }
                        },
                        Err(e) => {
                            // More detailed error logging
                            if e.contains("No valid operations found")
                                || e.contains("Skipping unsupported task command")
                                || e.contains("must be the first command")
                            {
                                // These might be benign (e.g. test file with only //# init or specific ordering issues)
                                debug_println_error!(
                                    "Skipping transactional test {}: {}",
                                    entry.path().display(),
                                    e
                                );
                            } else {
                                // eprintln is better for user-facing errors if this becomes a CLI tool part
                                // For fuzzer internal logic, debug_println_error or a dedicated logger is fine.
                                debug_println_error!(
                                    "Error processing transactional test {}: {}",
                                    entry.path().display(),
                                    e
                                );
                            }
                        },
                    }
                }
            }
        }

        // Process Move projects recursively from common test directories
        let move_test_dirs = vec![
            //"../../aptos-move/move-examples",
            //"../../aptos-move/e2e-move-tests",
        ];

        for test_dir in move_test_dirs {
            if std::path::Path::new(test_dir).exists() {
                println!("Processing Move projects from: {}", test_dir);
                if let Err(e) = generate_runnable_states_recursive(test_dir, destination_path) {
                    debug_println_error!("Failed to process {}: {}", test_dir, e);
                }
            }
        }
        Ok(())
    }
}
