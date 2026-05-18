// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Test and benchmark helpers for running programs against the micro-op
//! interpreter and the Move VM.

// ---------------------------------------------------------------------------
// Move VM helpers
// ---------------------------------------------------------------------------

use move_binary_format::file_format::CompiledModule;
use move_core_types::{identifier::Identifier, value::MoveValue};
use move_vm_runtime::{
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::{MoveVM, SerializedReturnValues},
    native_extensions::NativeContextExtensions,
    AsUnsyncModuleStorage, InstantiatedFunctionLoader, LazyLoader, LegacyLoaderConfig,
    LoadedFunction,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::gas::UnmeteredGasMeter;

/// Publish a compiled module into an in-memory storage.
pub fn publish_module(module: &CompiledModule) -> InMemoryStorage {
    publish_modules(&[module])
}

/// Publish multiple compiled modules into an in-memory storage.
pub fn publish_modules(modules: &[&CompiledModule]) -> InMemoryStorage {
    let mut storage = InMemoryStorage::new();
    for module in modules {
        let mut blob = vec![];
        module.serialize(&mut blob).unwrap();
        storage.add_module_bytes(module.self_addr(), module.self_name(), blob.into());
    }
    storage
}

/// Execute a Move function by name with the given serialized arguments.
///
/// Handles all VM setup internally. Best for tests where you don't need
/// fine-grained control over what's measured.
pub fn run_move_function(
    module: &CompiledModule,
    fun_name: &str,
    args: Vec<Vec<u8>>,
) -> SerializedReturnValues {
    let storage = publish_module(module);
    let module_storage = storage.as_unsync_module_storage();

    let module_id = module.self_id();
    let fun_name = Identifier::new(fun_name).unwrap();

    let mut data_cache = TransactionDataCache::empty();
    let mut gas_meter = UnmeteredGasMeter;
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);

    let loader = LazyLoader::new(&module_storage);
    let func = loader
        .load_instantiated_function(
            &LegacyLoaderConfig::unmetered(),
            &mut gas_meter,
            &mut traversal_context,
            &module_id,
            &fun_name,
            &[],
        )
        .unwrap();
    MoveVM::execute_loaded_function(
        func,
        args,
        &mut MoveVmDataCacheAdapter::new(&mut data_cache, &storage, &loader),
        &mut gas_meter,
        &mut traversal_context,
        &mut NativeContextExtensions::default(),
        &loader,
    )
    .unwrap()
}

/// Publish a module, load a function, and pass a [`LoadedMoveFunction`]
/// to a closure for repeated execution.
///
/// All setup (module publishing, function loading, per-execution state
/// allocation) happens once before the closure is called. Inside the
/// closure, only [`LoadedMoveFunction::run`] executes — just the VM
/// interpreter.
///
/// # Example
/// ```ignore
/// with_loaded_move_function(&module, "fib", |env| {
///     b.iter(|| {
///         let result = env.run(vec![arg_u64(25)]);
///         black_box(return_u64(&result))
///     });
/// });
/// ```
pub fn with_loaded_move_function<R>(
    module: &CompiledModule,
    fun_name: &str,
    f: impl FnOnce(&mut LoadedMoveFunction<'_>) -> R,
) -> R {
    with_loaded_move_function_with_deps(module, &[], fun_name, f)
}

/// Like [`with_loaded_move_function`], but also publishes dependency modules
/// (e.g., stdlib vector) that the main module references at runtime.
pub fn with_loaded_move_function_with_deps<R>(
    module: &CompiledModule,
    deps: &[&CompiledModule],
    fun_name: &str,
    f: impl FnOnce(&mut LoadedMoveFunction<'_>) -> R,
) -> R {
    let mut all_modules: Vec<&CompiledModule> = deps.to_vec();
    all_modules.push(module);
    let storage = publish_modules(&all_modules);
    let module_storage = storage.as_unsync_module_storage();
    let loader = LazyLoader::new(&module_storage);

    let module_id = module.self_id();
    let fun_name = Identifier::new(fun_name).unwrap();
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);
    let mut gas_meter = UnmeteredGasMeter;

    let func = loader
        .load_instantiated_function(
            &LegacyLoaderConfig::unmetered(),
            &mut gas_meter,
            &mut traversal_context,
            &module_id,
            &fun_name,
            &[],
        )
        .unwrap();

    // Per-execution state — allocated once, reused across calls.
    let mut data_cache = TransactionDataCache::empty();
    let mut gas_meter = UnmeteredGasMeter;
    let traversal_storage = TraversalStorage::new();
    let mut extensions = NativeContextExtensions::default();

    let mut env = LoadedMoveFunction {
        func,
        execute_fn: &mut |func, args| {
            let mut traversal_context = TraversalContext::new(&traversal_storage);
            MoveVM::execute_loaded_function(
                func,
                args,
                &mut MoveVmDataCacheAdapter::new(&mut data_cache, &storage, &loader),
                &mut gas_meter,
                &mut traversal_context,
                &mut extensions,
                &loader,
            )
            .unwrap()
        },
    };
    f(&mut env)
}

/// Pre-loaded Move function for repeated execution.
///
/// Created by [`with_loaded_move_function`]. Only the VM interpreter
/// runs inside [`run`](Self::run).
pub struct LoadedMoveFunction<'a> {
    func: LoadedFunction,
    execute_fn: &'a mut dyn FnMut(LoadedFunction, Vec<Vec<u8>>) -> SerializedReturnValues,
}

impl LoadedMoveFunction<'_> {
    /// Execute the loaded function with the given serialized arguments.
    pub fn run(&mut self, args: Vec<Vec<u8>>) -> SerializedReturnValues {
        (self.execute_fn)(self.func.clone(), args)
    }
}

// ---------------------------------------------------------------------------
// Argument serialization helpers
// ---------------------------------------------------------------------------

pub fn arg_u64(n: u64) -> Vec<u8> {
    MoveValue::U64(n).simple_serialize().unwrap()
}

pub fn arg_vec_u64(v: &[u64]) -> Vec<u8> {
    let elems: Vec<MoveValue> = v.iter().map(|&x| MoveValue::U64(x)).collect();
    MoveValue::Vector(elems).simple_serialize().unwrap()
}

// ---------------------------------------------------------------------------
// Return value extraction helpers
// ---------------------------------------------------------------------------

pub fn return_u64(result: &SerializedReturnValues) -> u64 {
    let bytes = &result.return_values[0].0;
    u64::from_le_bytes(bytes[..8].try_into().unwrap())
}

pub fn return_vec_u64(result: &SerializedReturnValues) -> Vec<u64> {
    let bytes = &result.return_values[0].0;
    let layout = &result.return_values[0].1;
    let val = MoveValue::simple_deserialize(bytes, layout).unwrap();
    match val {
        MoveValue::Vector(elems) => elems
            .into_iter()
            .map(|e| match e {
                MoveValue::U64(v) => v,
                _ => panic!("expected U64 element"),
            })
            .collect(),
        _ => panic!("expected Vector return value"),
    }
}
