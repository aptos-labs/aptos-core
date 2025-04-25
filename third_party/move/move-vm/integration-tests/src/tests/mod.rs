// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::compiler::{as_module, compile_units};
use move_binary_format::errors::{Location, VMResult};
use move_core_types::{
    effects::ChangeSet,
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
};
use move_vm_runtime::{
    data_cache::TransactionDataCache,
    dispatch_loader,
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::{MoveVM, SerializedReturnValues},
    native_extensions::NativeContextExtensions,
    AsUnsyncCodeStorage, AsUnsyncModuleStorage, CodeStorage, InstantiatedFunctionLoader,
    LegacyLoaderConfig, ModuleStorage,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{gas::UnmeteredGasMeter, resolver::ResourceResolver};

mod bad_entry_point_tests;
mod bad_storage_tests;
mod binary_format_version;
mod exec_func_effects_tests;
mod function_arg_tests;
mod instantiation_tests;
mod invariant_violation_tests;
mod leak_tests;
mod loader_tests;
mod module_storage_tests;
mod native_tests;
mod nested_loop_tests;
mod regression_tests;
mod return_value_tests;
mod runtime_reentrancy_check_tests;
mod vm_arguments_tests;

/// Given code string, compiles it, serializes and adds to storage.
fn compile_and_publish(storage: &mut InMemoryStorage, code: String) {
    let mut units = compile_units(&code).unwrap();
    let m = as_module(units.pop().unwrap());
    let mut blob = vec![];
    m.serialize(&mut blob).unwrap();
    storage.add_module_bytes(m.self_addr(), m.self_name(), blob.into());
}

/// Executes a Move script on top of the provided storage state, with the given arguments and type
/// arguments.
fn execute_script_for_test(
    storage: &InMemoryStorage,
    script: &[u8],
    ty_args: &[TypeTag],
    args: Vec<Vec<u8>>,
) -> VMResult<()> {
    execute_script_impl(storage, script, ty_args, args)?;
    Ok(())
}

/// Executes a Move script on top of the provided storage state, and commits changes to resources
/// to the storage.
fn execute_script_and_commit_change_set_for_test(
    storage: &mut InMemoryStorage,
    script: &[u8],
    ty_args: &[TypeTag],
    args: Vec<Vec<u8>>,
) -> VMResult<()> {
    let change_set = execute_script_impl(storage, script, ty_args, args)?;
    storage
        .apply(change_set)
        .map_err(|err| err.finish(Location::Undefined))?;
    Ok(())
}

fn execute_function_with_single_storage_for_test(
    storage: &InMemoryStorage,
    module_id: &ModuleId,
    function_name: &IdentStr,
    ty_args: &[TypeTag],
    args: Vec<Vec<u8>>,
) -> VMResult<SerializedReturnValues> {
    let module_storage = storage.as_unsync_module_storage();
    execute_function_for_test(
        storage,
        &module_storage,
        module_id,
        function_name,
        ty_args,
        args,
    )
}

fn execute_function_for_test(
    data_storage: &impl ResourceResolver,
    module_storage: &impl ModuleStorage,
    module_id: &ModuleId,
    function_name: &IdentStr,
    ty_args: &[TypeTag],
    args: Vec<Vec<u8>>,
) -> VMResult<SerializedReturnValues> {
    let mut gas_meter = UnmeteredGasMeter;
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);

    dispatch_loader!(module_storage, loader, {
        let func = loader.load_instantiated_function(
            &LegacyLoaderConfig::unmetered(),
            &mut gas_meter,
            &mut traversal_context,
            module_id,
            function_name,
            ty_args,
        )?;
        MoveVM::execute_loaded_function(
            func,
            args,
            &mut TransactionDataCache::empty(),
            &mut gas_meter,
            &mut traversal_context,
            &mut NativeContextExtensions::default(),
            &loader,
            data_storage,
        )
    })
}

fn execute_script_impl(
    storage: &InMemoryStorage,
    script: &[u8],
    ty_args: &[TypeTag],
    args: Vec<Vec<u8>>,
) -> VMResult<ChangeSet> {
    let code_storage = storage.as_unsync_code_storage();
    let traversal_storage = TraversalStorage::new();

    let function = code_storage.load_script(script, ty_args)?;
    let mut data_cache = TransactionDataCache::empty();

    dispatch_loader!(&code_storage, loader, {
        MoveVM::execute_loaded_function(
            function,
            args,
            &mut data_cache,
            &mut UnmeteredGasMeter,
            &mut TraversalContext::new(&traversal_storage),
            &mut NativeContextExtensions::default(),
            &loader,
            storage,
        )
    })?;

    let change_set = data_cache
        .into_effects(&code_storage)
        .map_err(|err| err.finish(Location::Undefined))?;
    Ok(change_set)
}
