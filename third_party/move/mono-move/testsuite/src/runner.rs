// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Executes parsed test steps against both MoveVM and mono-move, producing
//! normalized output for comparison.

use crate::{matcher::check_output, parser::Step};
use anyhow::anyhow;
use legacy_move_compiler::{compiled_unit::CompiledUnit, shared::known_attributes::KnownAttribute};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_runtime::InterpreterContext;
use move_binary_format::CompiledModule;
use move_compiler_v2::{run_move_compiler_to_stderr, Options};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::ModuleId,
    value::MoveValue,
};
use move_vm_runtime::{
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::MoveVM,
    native_extensions::NativeContextExtensions,
    AsUnsyncModuleStorage, InstantiatedFunctionLoader, LazyLoader, LegacyLoaderConfig,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{gas::UnmeteredGasMeter, loaded_data::runtime_types::Type};
use std::path::Path;

/// Execution output from a VM, carrying both the display string and the
/// number of return values so that mono-move can avoid reparsing.
struct Output {
    display: String,
    num_returns: usize,
}

/// Run all steps in a differential test, checking both VMs produce matching output.
pub fn run_test(steps: Vec<Step>) -> anyhow::Result<()> {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let mut storage = InMemoryStorage::new();

    for step in steps {
        match step {
            Step::Publish { sources } => {
                let modules = compile_move_modules(&sources);
                for module in &modules {
                    // V1 path.
                    let mut blob = vec![];
                    module
                        .serialize(&mut blob)
                        .map_err(|err| anyhow!("Failed to serialize module: {}", err))?;
                    // Directly insert into in-memory storage rather than going
                    // through the full publishing workflow (compatibility checks,
                    // etc.) — sufficient for differential testing.
                    storage.add_module_bytes(module.self_addr(), module.self_name(), blob.into());

                    // V2 path.
                    let id = guard.intern_address_name(module.self_addr(), module.self_name());
                    let executable = guard
                        .executable_builder_for_module(module)
                        .build()
                        .map_err(|err| anyhow!("Failed to build executable: {}", err))?;
                    guard.insert_executable(id, executable);
                }
            },
            Step::Execute {
                address,
                module_name,
                function_name,
                args,
                checks,
            } => {
                let v1_output =
                    execute_function_v1(&storage, &address, &module_name, &function_name, &args);
                let v2_output = execute_function_v2(
                    &guard,
                    &address,
                    &module_name,
                    &function_name,
                    &args,
                    v1_output.num_returns,
                );
                check_output(&checks, &v1_output.display, &v2_output.display)?;
            },
        }
    }

    Ok(())
}

/// Compile Move sources into binary format.
pub fn compile_move_modules(sources: &str) -> Vec<CompiledModule> {
    use std::io::Write;

    let tmp_dir = tempfile::tempdir().expect("Should always be able to create temporary directory");
    let path = tmp_dir.path().join("sources.move");
    std::fs::File::create(&path)
        .and_then(|mut f| f.write_all(sources.as_bytes()))
        .expect("failed to write temp source file");

    let options = compiler_options(&path);
    run_move_compiler_to_stderr(options)
        .unwrap_or_else(|err| panic!("Move compilation failed: {}", err))
        .1
        .into_iter()
        .map(|unit| match unit.into_compiled_unit() {
            CompiledUnit::Module(m) => m.module,
            CompiledUnit::Script(_) => unreachable!("Only modules can be published"),
        })
        .collect()
}

/// Returns default compiler options used in the test.
fn compiler_options(sources_path: &Path) -> Options {
    Options {
        sources: vec![sources_path.to_string_lossy().into_owned()],
        dependencies: vec![],
        named_address_mapping: vec![],
        known_attributes: KnownAttribute::get_all_attribute_names().clone(),
        ..Options::default()
    }
}

/// Execute a function via legacy MoveVM and returns normalized output.
fn execute_function_v1(
    storage: &InMemoryStorage,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    args: &[String],
) -> Output {
    let mut gas_meter = UnmeteredGasMeter;

    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);

    let module_storage = storage.as_unsync_module_storage();
    let loader = LazyLoader::new(&module_storage);

    let function = match loader.load_instantiated_function(
        &LegacyLoaderConfig::unmetered(),
        &mut gas_meter,
        &mut traversal_context,
        &ModuleId::new(*address, module_name.to_owned()),
        function_name,
        // TODO: support type arguments.
        &[],
    ) {
        Ok(function) => function,
        Err(err) => {
            // For testing purposes, loading function should always succeed.
            panic!("Failed to load function: {}", err)
        },
    };

    if function.param_tys().len() != args.len() {
        panic!("Function requires a different number of arguments");
    }
    let args = function
        .param_tys()
        .iter()
        .zip(args.iter())
        .map(|(ty, arg)| match ty {
            Type::U64 => {
                let value = arg.parse::<u64>().expect("Argument must be a valid u64");
                MoveValue::U64(value).simple_serialize().unwrap()
            },
            _ => unimplemented!("Only u64 argument types are supported"),
        })
        .collect::<Vec<_>>();

    let mut data_cache = TransactionDataCache::empty();
    match MoveVM::execute_loaded_function(
        function,
        args,
        &mut MoveVmDataCacheAdapter::new(&mut data_cache, storage, &loader),
        &mut gas_meter,
        &mut traversal_context,
        &mut NativeContextExtensions::default(),
        &loader,
    ) {
        Ok(result) => {
            let num_returns = result.return_values.len();
            let vals = result
                .return_values
                .iter()
                .map(|(bytes, _layout)| {
                    let val = u64::from_le_bytes(bytes[..8].try_into().unwrap());
                    val.to_string()
                })
                .collect::<Vec<_>>();
            Output {
                display: format!("results: {}", vals.join(", ")),
                num_returns,
            }
        },
        Err(err) => Output {
            display: format!("error: {}", err),
            num_returns: 0,
        },
    }
}

/// Executes a function via MonoMove VM, and returns normalized output.
fn execute_function_v2(
    guard: &ExecutionGuard<'_>,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    args: &[String],
    // TODO: Remove once function carries type signature.
    num_returns: usize,
) -> Output {
    // Look up the executable from the context's cache.
    let id = guard.intern_address_name(address, module_name);
    let function_name = guard.intern_identifier(function_name);
    let function = guard
        .get_executable(id)
        .and_then(|executable| executable.get_function(function_name))
        .unwrap_or_else(|| panic!("Failed to load function or find executable"));

    // TODO: Set object descriptor table when supported.
    let mut interpreter = InterpreterContext::new(&[], function);

    // TODO: Check function signature to decide how to parse arguments.
    for (i, arg) in args.iter().enumerate() {
        let arg = arg
            .parse::<u64>()
            .expect("Only u64 arguments are supported");
        interpreter.set_root_arg((i * 8) as u32, &arg.to_le_bytes());
    }

    match interpreter.run() {
        Err(err) => Output {
            display: format!("error: {}", err),
            num_returns: 0,
        },
        Ok(()) => {
            if num_returns == 0 {
                // TODO: Check frame contents?
                Output {
                    display: "results:".to_string(),
                    num_returns: 0,
                }
            } else {
                let vals = (0..num_returns)
                    .map(|i| interpreter.root_result_at((i * 8) as u32).to_string())
                    .collect::<Vec<_>>();
                Output {
                    display: format!("results: {}", vals.join(", ")),
                    num_returns,
                }
            }
        },
    }
}
