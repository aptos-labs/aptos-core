// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the bound on per-instruction frame caches.
//!
//! A frame cache is created per distinct `(function, type arguments)` pair and its per-instruction
//! cache is sized by the callee's bytecode length. Both quantities are user controlled, so the
//! product is bounded by [`move_vm_runtime::InstructionCacheBudgetOverride`]'s budget.
//!
//! The bound itself is asserted deterministically by the unit tests in
//! `move-vm-runtime/src/frame_type_cache.rs`. What these tests establish is the property the bound
//! relies on to ship without a feature gate: the per-instruction cache is a pure memoization of
//! the frame cache's function maps, so varying the budget - including removing the cache entirely -
//! cannot change execution results or gas.

use move_binary_format::{
    errors::VMResult,
    file_format::{
        empty_module, Bytecode, CodeUnit, CompiledModule, FieldDefinition, FunctionDefinition,
        FunctionHandle, FunctionHandleIndex, FunctionInstantiation, FunctionInstantiationIndex,
        IdentifierIndex, ModuleHandleIndex, Signature, SignatureIndex, SignatureToken,
        StructDefinition, StructFieldInformation, StructHandle, StructHandleIndex, TypeSignature,
        Visibility,
    },
};
use move_core_types::{
    ability::{Ability, AbilitySet},
    account_address::AccountAddress,
    function::ClosureMask,
    ident_str,
    identifier::{IdentStr, Identifier},
    vm_status::StatusCode,
};
use move_vm_runtime::{
    config::VMConfig,
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    dispatch_loader,
    execution_tracing::{FullTraceRecorder, NoOpTraceRecorder, Trace, TraceRecorder},
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::{MoveVM, SerializedReturnValues},
    native_extensions::NativeContextExtensions,
    AsUnsyncModuleStorage, InstantiatedFunctionLoader, InstructionCacheBudgetOverride,
    LegacyLoaderConfig, ModuleStorage, RuntimeEnvironment, TypeChecker,
};
use move_vm_test_utils::{
    gas_schedule::{Gas, GasStatus, INITIAL_COST_SCHEDULE},
    InMemoryStorage,
};

const ADDRESS: AccountAddress = AccountAddress::TWO;

/// Number of distinct single-field structs used to build type arguments. Every ordered pair is a
/// distinct instantiation of `pad`, so this produces `STRUCT_COUNT * STRUCT_COUNT` frame caches.
const STRUCT_COUNT: usize = 23;
const INSTANTIATIONS: usize = STRUCT_COUNT * STRUCT_COUNT;

/// Bytecode length of the callee. Only the first instruction executes; the rest is padding that a
/// publisher gets for free, since `Nop` costs nothing in the binary complexity meter and the
/// bytecode verifier skips unreachable blocks. Keep this small: the `usize::MAX`-budget runs
/// allocate the full `INSTANTIATIONS * PAD_CODE_SIZE` product (~12 MiB here), and a larger pad
/// proves nothing more — the blow-up is scale-invariant.
const PAD_CODE_SIZE: usize = 1_001;

/// `call_all` is one `CallGeneric` per instantiation plus a `Ret`. Its own frame cache is allocated
/// too, so it counts towards the budget.
const CALL_ALL_CODE_SIZE: usize = INSTANTIATIONS + 1;

/// Entries the pre-fix code allocated for this call graph: one full-length per-instruction cache
/// per distinct instantiation, plus the entry function's own.
const UNBOUNDED_ENTRIES: usize = INSTANTIATIONS * PAD_CODE_SIZE + CALL_ALL_CODE_SIZE;

/// What an execution produced. `call_all` returns nothing, so the status and the gas consumed fully
/// describe the observable outcome.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    status: Option<StatusCode>,
    gas_used: u64,
}

fn push_identifier(module: &mut CompiledModule, name: &str) -> IdentifierIndex {
    let index = module.identifiers.len() as u16;
    module.identifiers.push(Identifier::new(name).unwrap());
    IdentifierIndex(index)
}

/// Pushes a handle for a function with no parameters and no return values.
fn push_function_handle(
    module: &mut CompiledModule,
    name: &str,
    type_parameters: Vec<AbilitySet>,
) -> FunctionHandleIndex {
    let name = push_identifier(module, name);
    let index = module.function_handles.len() as u16;
    module.function_handles.push(FunctionHandle {
        module: ModuleHandleIndex(0),
        name,
        parameters: SignatureIndex(0),
        return_: SignatureIndex(0),
        type_parameters,
        access_specifiers: None,
        attributes: vec![],
    });
    FunctionHandleIndex(index)
}

/// The first `Ret` makes everything after it unreachable, so the padding is never executed and
/// never analyzed by the verifier's abstract interpreter.
fn padded_code(code_size: usize) -> Vec<Bytecode> {
    let mut code = vec![Bytecode::Ret];
    code.extend(std::iter::repeat_n(Bytecode::Nop, code_size - 2));
    code.push(Bytecode::Ret);
    assert_eq!(code.len(), code_size);
    code
}

/// Builds a storage with the module published and paranoid type checks enabled.
fn storage_with_module(module: &CompiledModule) -> InMemoryStorage {
    let runtime_environment = RuntimeEnvironment::new_with_config(vec![], VMConfig {
        paranoid_type_checks: true,
        ..VMConfig::default_for_test()
    });
    let mut storage = InMemoryStorage::new_with_runtime_environment(runtime_environment);

    let mut module_bytes = vec![];
    module.serialize(&mut module_bytes).unwrap();
    storage.add_module_bytes(module.self_addr(), module.self_name(), module_bytes.into());
    storage
}

/// Builds a module with the shape: one long generic callee reached at many
/// distinct type vectors.
///
///
/// ```text
/// module 0x2::m {
///     struct S0 has copy, drop, store { value: u8 }
///     ...
///     public fun pad<T0, T1>() { return; <PAD_CODE_SIZE - 2 nops>; return }
///     public fun call_all() { pad<S0, S0>(); pad<S0, S1>(); ...; return }
/// }
/// ```
fn many_instantiations_module() -> CompiledModule {
    let mut module = empty_module();
    module.address_identifiers[0] = ADDRESS;
    module.identifiers[0] = Identifier::new("m").unwrap();

    let pad_handle = push_function_handle(&mut module, "pad", vec![
        AbilitySet::EMPTY,
        AbilitySet::EMPTY,
    ]);
    let call_all_handle = push_function_handle(&mut module, "call_all", vec![]);

    let field_name = push_identifier(&mut module, "value");
    let abilities = AbilitySet::singleton(Ability::Copy) | Ability::Drop | Ability::Store;
    for index in 0..STRUCT_COUNT {
        let struct_name = push_identifier(&mut module, &format!("S{index}"));
        module.struct_handles.push(StructHandle {
            module: ModuleHandleIndex(0),
            name: struct_name,
            abilities,
            type_parameters: vec![],
        });
        module.struct_defs.push(StructDefinition {
            struct_handle: StructHandleIndex(index as u16),
            field_information: StructFieldInformation::Declared(vec![FieldDefinition {
                name: field_name,
                signature: TypeSignature(SignatureToken::U8),
            }]),
        });
    }

    module.function_defs.push(FunctionDefinition {
        function: pad_handle,
        visibility: Visibility::Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: padded_code(PAD_CODE_SIZE),
        }),
    });

    let mut call_all_code = Vec::with_capacity(INSTANTIATIONS + 1);
    for first in 0..STRUCT_COUNT {
        for second in 0..STRUCT_COUNT {
            let signature_index = module.signatures.len() as u16;
            module.signatures.push(Signature(vec![
                SignatureToken::Struct(StructHandleIndex(first as u16)),
                SignatureToken::Struct(StructHandleIndex(second as u16)),
            ]));

            let instantiation_index = module.function_instantiations.len() as u16;
            module.function_instantiations.push(FunctionInstantiation {
                handle: pad_handle,
                type_parameters: SignatureIndex(signature_index),
            });
            call_all_code.push(Bytecode::CallGeneric(FunctionInstantiationIndex(
                instantiation_index,
            )));
        }
    }
    call_all_code.push(Bytecode::Ret);

    module.function_defs.push(FunctionDefinition {
        function: call_all_handle,
        visibility: Visibility::Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: call_all_code,
        }),
    });

    module
}

/// Executes `0x2::m::call_all` with the per-instruction cache budget capped at `max_cache_entries`,
/// returning the observable outcome and the number of cache entries the run allocated.
fn run_call_all(module: &CompiledModule, max_cache_entries: usize) -> (Outcome, usize) {
    let budget_override = InstructionCacheBudgetOverride::new(max_cache_entries);

    let storage = storage_with_module(module);
    let module_storage = storage.as_unsync_module_storage();
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);
    let mut data_cache = TransactionDataCache::empty();

    let gas_budget = Gas::new(1_000_000);
    let mut gas_meter = GasStatus::new(INITIAL_COST_SCHEDULE.clone(), gas_budget);

    let result = execute_function(
        module,
        ident_str!("call_all"),
        &module_storage,
        &storage,
        &mut data_cache,
        &mut gas_meter,
        &mut traversal_context,
        &mut NoOpTraceRecorder,
    );

    let outcome = Outcome {
        status: result.err().map(|err| err.major_status()),
        gas_used: u64::from(gas_budget) - u64::from(gas_meter.remaining_gas()),
    };
    (outcome, budget_override.entries_allocated())
}

fn execute_function(
    module: &CompiledModule,
    function_name: &IdentStr,
    module_storage: &impl ModuleStorage,
    storage: &InMemoryStorage,
    data_cache: &mut TransactionDataCache,
    gas_meter: &mut GasStatus,
    traversal_context: &mut TraversalContext,
    recorder: &mut impl TraceRecorder,
) -> VMResult<SerializedReturnValues> {
    dispatch_loader!(module_storage, loader, {
        let function = loader.load_instantiated_function(
            &LegacyLoaderConfig::unmetered(),
            gas_meter,
            traversal_context,
            &module.self_id(),
            function_name,
            &[],
        )?;
        MoveVM::execute_loaded_function_with_tracing(
            function,
            Vec::<Vec<u8>>::new(),
            &mut MoveVmDataCacheAdapter::new(data_cache, storage, &loader),
            gas_meter,
            traversal_context,
            &mut NativeContextExtensions::default(),
            &loader,
            recorder,
        )
    })
}

/// The property that lets the bound ship without a feature gate.
///
/// The per-instruction cache is only ever written after the corresponding entry exists in
/// `FrameTypeCache::function_cache` / `generic_function_cache`, and gas is charged only when those
/// maps miss. A hit in the per-instruction cache therefore implies a hit in the maps, so removing
/// it can never cause work to be redone or gas to be charged twice.
#[test]
fn cache_budget_does_not_change_results_or_gas() {
    let module = many_instantiations_module();
    move_bytecode_verifier::verify_module(&module).expect("module must verify");

    // No per-instruction caching at all: every call site falls through to the function maps.
    let (without_cache, entries) = run_call_all(&module, 0);
    assert_eq!(
        without_cache.status, None,
        "execution must succeed, got {:?}",
        without_cache.status
    );
    assert_eq!(entries, 0);
    assert!(
        without_cache.gas_used > 0,
        "gas must actually be metered for this comparison to mean anything"
    );

    // Enough budget for every instantiation, i.e. the unbounded pre-fix allocation.
    let (with_full_cache, entries) = run_call_all(&module, usize::MAX);
    assert_eq!(entries, UNBOUNDED_ENTRIES);
    assert_eq!(with_full_cache, without_cache);

    // Partial: the budget runs out partway through, so some frame caches have a per-instruction
    // cache and some do not. This is the state the production bound actually produces.
    let max_entries = PAD_CODE_SIZE * 3;
    let (with_partial_cache, entries) = run_call_all(&module, max_entries);
    assert!((1..=max_entries).contains(&entries), "allocated {entries}");
    assert_eq!(with_partial_cache, without_cache);

    // A budget too small for the padded callee, so only the entry function gets a cache.
    let max_entries = PAD_CODE_SIZE - 1;
    let (with_unusable_cache, entries) = run_call_all(&module, max_entries);
    assert_eq!(entries, CALL_ALL_CODE_SIZE);
    assert_eq!(with_unusable_cache, without_cache);
}

/// Regression test for APTOSNT-487.
///
/// Before the fix, `FrameTypeCache::make_rc_for_function` resized the per-instruction cache to the
/// callee's full bytecode length unconditionally and every distinct instantiation got its own, so
/// this call graph retained `INSTANTIATIONS * PAD_CODE_SIZE` entries for a transaction that
/// executes `INSTANTIATIONS + 1` instructions. The unbounded run below reproduces that number
/// exactly; scaled to mainnet's transaction and dependency limits the same shape reaches ~190 GiB.
#[test]
fn many_instantiations_of_a_long_function_stay_within_budget() {
    let module = many_instantiations_module();
    move_bytecode_verifier::verify_module(&module).expect("module must verify");

    // Pre-fix behaviour: allocation is the product of two user-controlled quantities.
    let (unbounded_outcome, unbounded_entries) = run_call_all(&module, usize::MAX);
    assert_eq!(unbounded_outcome.status, None);
    assert_eq!(unbounded_entries, UNBOUNDED_ENTRIES);

    // Post-fix: the same call graph is capped at the budget, and still runs.
    let max_entries = PAD_CODE_SIZE * 2;
    let (bounded_outcome, bounded_entries) = run_call_all(&module, max_entries);
    assert_eq!(bounded_outcome.status, None, "execution must succeed");
    assert!(
        bounded_entries <= max_entries,
        "allocated {bounded_entries} entries against a budget of {max_entries}"
    );
    assert!(
        unbounded_entries / bounded_entries >= 100,
        "bound must be a material reduction: {unbounded_entries} -> {bounded_entries}"
    );
}

/// Number of times `call_repeatedly` packs and calls a closure over the same callee.
const CLOSURE_CALLS: usize = 100;

/// `call_repeatedly` is one `PackClosure` + `CallClosure` pair per call plus a `Ret`.
const CLOSURE_CALLER_CODE_SIZE: usize = 2 * CLOSURE_CALLS + 1;

/// Bytecode length of the closure callee, padded like `pad` so that a fresh cache per call would
/// dominate the entry count.
const CLOSURE_CALLEE_CODE_SIZE: usize = 1_001;

/// Builds a module that calls the same function through a closure many times:
///
/// ```text
/// module 0x2::n {
///     fun callee() { return; <padding>; return }
///     public fun call_repeatedly() { (|| callee())(); ...; return }
/// }
/// ```
fn repeated_closure_calls_module() -> CompiledModule {
    let mut module = empty_module();
    module.address_identifiers[0] = ADDRESS;
    module.identifiers[0] = Identifier::new("n").unwrap();

    let callee_handle = push_function_handle(&mut module, "callee", vec![]);
    let caller_handle = push_function_handle(&mut module, "call_repeatedly", vec![]);

    // `PackClosure` of a function without the persistent attribute pushes `|| has copy + drop`,
    // and `CallClosure`'s signature must be assignable from that type.
    let closure_signature_index = SignatureIndex(module.signatures.len() as u16);
    module
        .signatures
        .push(Signature(vec![SignatureToken::Function(
            vec![],
            vec![],
            AbilitySet::PRIVATE_FUNCTIONS,
        )]));

    module.function_defs.push(FunctionDefinition {
        function: callee_handle,
        visibility: Visibility::Private,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: padded_code(CLOSURE_CALLEE_CODE_SIZE),
        }),
    });

    let mut caller_code = Vec::with_capacity(CLOSURE_CALLER_CODE_SIZE);
    for _ in 0..CLOSURE_CALLS {
        caller_code.push(Bytecode::PackClosure(callee_handle, ClosureMask::empty()));
        caller_code.push(Bytecode::CallClosure(closure_signature_index));
    }
    caller_code.push(Bytecode::Ret);
    assert_eq!(caller_code.len(), CLOSURE_CALLER_CODE_SIZE);

    module.function_defs.push(FunctionDefinition {
        function: caller_handle,
        visibility: Visibility::Public,
        is_entry: false,
        acquires_global_resources: vec![],
        code: Some(CodeUnit {
            locals: SignatureIndex(0),
            code: caller_code,
        }),
    });

    module
}

/// Executes `0x2::n::call_repeatedly` recording a trace. With a recorder installed, paranoid type
/// checks are deferred from execution to trace replay.
fn record_call_repeatedly_trace(
    module: &CompiledModule,
    module_storage: &impl ModuleStorage,
    storage: &InMemoryStorage,
) -> Trace {
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);
    let mut data_cache = TransactionDataCache::empty();
    let mut gas_meter = GasStatus::new(INITIAL_COST_SCHEDULE.clone(), Gas::new(1_000_000));

    let mut recorder = FullTraceRecorder::new();
    execute_function(
        module,
        ident_str!("call_repeatedly"),
        module_storage,
        storage,
        &mut data_cache,
        &mut gas_meter,
        &mut traversal_context,
        &mut recorder,
    )
    .expect("execution must succeed");
    recorder.finish()
}

/// Replays the trace with the per-instruction cache budget capped at `max_cache_entries`,
/// returning the replay result and the number of cache entries the replay allocated. The override
/// is installed here, after recording, so the counter sees replay allocations alone and the
/// budget is snapshotted when the type checker is created.
fn replay_with_budget(
    module_storage: &impl ModuleStorage,
    trace: &Trace,
    max_cache_entries: usize,
) -> (VMResult<()>, usize) {
    let budget_override = InstructionCacheBudgetOverride::new(max_cache_entries);
    let result = TypeChecker::new(module_storage).replay(trace);
    (result, budget_override.entries_allocated())
}

/// Replay takes closure frame caches from the same memoized map as every other call path, so many
/// closure calls of one callee charge the budget for that callee once, not per call.
#[test]
fn replayed_closure_calls_share_one_frame_cache() {
    let module = repeated_closure_calls_module();
    move_bytecode_verifier::verify_module(&module).expect("module must verify");

    let storage = storage_with_module(&module);
    let module_storage = storage.as_unsync_module_storage();
    let trace = record_call_repeatedly_trace(&module, &module_storage, &storage);
    assert_eq!(trace.num_recorded_calls(), CLOSURE_CALLS + 1);

    // A fresh cache per closure call would charge CLOSURE_CALLS * CLOSURE_CALLEE_CODE_SIZE.
    let (result, entries) = replay_with_budget(&module_storage, &trace, usize::MAX);
    result.expect("replay must succeed");
    assert_eq!(entries, CLOSURE_CALLER_CODE_SIZE + CLOSURE_CALLEE_CODE_SIZE);

    // With no budget, the closure path degrades to uncached lookups like every other call path.
    let (result, entries) = replay_with_budget(&module_storage, &trace, 0);
    result.expect("replay must succeed with a zero budget");
    assert_eq!(entries, 0);
}
