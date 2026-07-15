// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Executes parsed test steps against both MoveVM and mono-move, producing
//! normalized output for comparison.

use crate::{
    compile::{compile, compile_move_path, compile_move_stdlib, SourceKind},
    engine::RunResult,
    extensions::{
        seed_extensions, TEST_SESSION_COUNTER, TEST_STATE_BYTES, TEST_STATE_ITEMS, TEST_TXN_HASH,
        TEST_TXN_INDEX,
    },
    matcher::check_output,
    module_provider::InMemoryModuleProvider,
    parser::{Check, PrintSection, Step},
    print_sections,
};
use anyhow::{anyhow, bail};
use aptos_framework_natives::{
    event::NativeEventContext, object::NativeObjectContext,
    state_storage::NativeStateStorageContext, transaction_context::NativeTransactionContext,
};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::{
    contract_event::ContractEvent,
    event::EventKey,
    on_chain_config::{Features, TimedFeaturesBuilder},
    state_store::{
        errors::StateViewError, state_key::StateKey, state_storage_usage::StateStorageUsage,
        StateViewId,
    },
    transaction::user_transaction_context::{TransactionIndexKind, UserTransactionContext},
};
use aptos_vm::natives::aptos_natives;
use aptos_vm_types::resolver::StateStorageView;
use mono_move_core::{native::NativeExtensions, types::type_to_string};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_natives::{EventKind, EventStore};
use mono_move_runtime::serialize;
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    int256::{I256, U256},
    language_storage::{ModuleId, TypeTag},
    value::MoveValue,
    vm_status::StatusCode,
};
use move_vm_runtime::{
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::MoveVM,
    native_extensions::NativeContextExtensions,
    native_functions::NativeFunctionTable,
    AsUnsyncModuleStorage, InstantiatedFunctionLoader, LazyLoader, LegacyLoaderConfig,
    RuntimeEnvironment, WithRuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{gas::UnmeteredGasMeter, loaded_data::runtime_types::Type};
use std::{path::Path, sync::OnceLock};

/// Execution output from a VM as a normalized display string.
struct Output {
    display: String,
}

/// A [`StateStorageView`] over empty storage that serves a fixed usage, for
/// exercising the legacy `state_storage` native in the differential tests.
//
// TODO(testing): replace with a real state view if tests need natives that read actual
// stored state.
struct MockEmptyStateStorage {
    usage: StateStorageUsage,
}

impl StateStorageView for MockEmptyStateStorage {
    type Key = StateKey;

    fn id(&self) -> StateViewId {
        StateViewId::Miscellaneous
    }

    fn read_state_value(&self, _state_key: &StateKey) -> Result<(), StateViewError> {
        Ok(())
    }

    fn get_usage(&self) -> Result<StateStorageUsage, StateViewError> {
        Ok(self.usage)
    }
}

/// Normalized rendering of one emitted event, used to diff the two VMs' events.
/// `creator`/`seq` are `Some` only for handle (V1) events.
fn render_event(
    creator: Option<AccountAddress>,
    seq: Option<u64>,
    type_str: &str,
    blob: &[u8],
) -> String {
    match (creator, seq) {
        (Some(creator), Some(seq)) => format!(
            "handle creator={} seq={} {} {}",
            creator.to_hex_literal(),
            seq,
            type_str,
            render_bytes(blob),
        ),
        _ => format!("module {} {}", type_str, render_bytes(blob)),
    }
}

/// Combines return values and rendered events into the normalized output string
/// that `CHECK` directives match against: a `results:` segment when the call
/// returns values and an `events:` segment when it emits events, joined by
/// ` | `. A void, event-free call renders as `results:`.
fn render_execution_output(vals: &[String], events: &[String]) -> String {
    let mut segments = Vec::new();
    if !vals.is_empty() {
        segments.push(format!("results: {}", vals.join(", ")));
    }
    if !events.is_empty() {
        segments.push(format!("events: {}", events.join("; ")));
    }
    if segments.is_empty() {
        return "results:".to_string();
    }
    segments.join(" | ")
}

/// Renders the events emitted into the legacy VM's [`NativeEventContext`], in
/// emission order, for cross-VM comparison.
pub fn finalize_events_v1(extensions: &NativeContextExtensions) -> Vec<String> {
    extensions
        .get::<NativeEventContext>()
        .events_iter()
        .map(|event| {
            let type_str = event.type_tag().to_canonical_string();
            match event {
                ContractEvent::V1(e) => render_event(
                    Some(e.key().get_creator_address()),
                    Some(e.sequence_number()),
                    &type_str,
                    e.event_data(),
                ),
                ContractEvent::V2(_) => render_event(None, None, &type_str, event.event_data()),
            }
        })
        .collect()
}

/// Serializes and renders the events recorded in mono-move's [`EventStore`], in
/// emission order, for cross-VM comparison. `layouts` (the guard's published
/// layouts) resolves the event types.
///
/// # Safety
///
/// The VM heap must be live: each entry's `msg_data` embeds heap pointers that
/// [`serialize`] dereferences.
pub unsafe fn finalize_events_v2(
    extensions: &NativeExtensions,
    layouts: &ExecutionGuard<'_>,
) -> Vec<String> {
    let store = extensions
        .get_mut::<EventStore>()
        .expect("event store installed");
    store
        .entries()
        .iter()
        .map(|entry| {
            let type_str = type_to_string(entry.msg_ty);
            // SAFETY: forwarded from this function's contract — the heap is live.
            let blob = unsafe { serialize(layouts, entry.msg_data.as_ptr(), entry.msg_ty) }
                .expect("event value serializes");
            match &entry.kind {
                EventKind::V2 => render_event(None, None, &type_str, &blob),
                EventKind::V1 {
                    guid,
                    sequence_number,
                } => {
                    let key: EventKey = bcs::from_bytes(guid).expect("guid decodes to EventKey");
                    render_event(
                        Some(key.get_creator_address()),
                        Some(*sequence_number),
                        &type_str,
                        &blob,
                    )
                },
            }
        })
        .collect()
}

/// Run all steps in a differential test, checking both VMs produce matching
/// output. If any `Publish` step requested `--print(...)` sections, the
/// rendered snapshot is verified against (or written to with `UPBL=1`) a
/// `.exp` baseline alongside `test_path`.
pub fn run_test(steps: Vec<Step>, kind: SourceKind, test_path: &Path) -> anyhow::Result<()> {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let runtime_env = RuntimeEnvironment::new(v1_native_table());
    let mut storage = InMemoryStorage::new_with_runtime_environment(runtime_env);
    let mut module_provider = InMemoryModuleProvider::new();
    let mut snapshot = String::new();

    // Publish the Move stdlib into both VMs so tests can call real stdlib
    // natives.
    for module in stdlib_modules().iter().chain(test_utils_modules()) {
        let mut blob = vec![];
        module.serialize(&mut blob).expect("module serializes");
        storage.add_module_bytes(module.self_addr(), module.self_name(), blob.into());
        module_provider.add_module(module);
    }

    for step in steps {
        match step {
            Step::Publish { sources, print } => {
                let modules = compile(&sources, kind)?;
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

                    // V2 path: stage the bytes; the loader builds executables
                    // lazily on first dispatch.
                    module_provider.add_module(module);
                }

                if !print.is_empty() {
                    if matches!(kind, SourceKind::Masm) && print.contains(&PrintSection::Bytecode) {
                        bail!(
                            "`bytecode` is not a valid print section for .masm inputs — \
                             the bytecode is the input"
                        );
                    }
                    snapshot.push_str(&print_sections::render(&guard, &modules, &print)?);
                }
            },
            Step::Execute {
                address,
                module_name,
                function_name,
                args,
                heap_size,
                checks,
            } => {
                // Run only the VM(s) a step actually checks (via `CHECK-V1` /
                // `CHECK-V2`). This lets a step assert just one side — needed for
                // natives one VM cannot run (e.g. `aggregator_v2`, which the
                // legacy harness installs no extension for and would panic on).
                // `CHECK-GC-COUNT` inspects the V2 collector, so it also needs V2.
                let needs_v1 = checks.iter().any(|c| matches!(c, Check::V1(..)));
                let needs_v2 = checks
                    .iter()
                    .any(|c| matches!(c, Check::V2(..) | Check::GcCount(..)));

                let v1 = needs_v1.then(|| {
                    execute_function_v1(&storage, &address, &module_name, &function_name, &args)
                });

                let (v2_display, v2_gc_count) = if needs_v2 {
                    // V2 needs the arg/return kinds. Reuse V1's if it ran,
                    // otherwise load the function (without running it) to size them.
                    let (param_kinds, return_kinds) = match &v1 {
                        Some(v1) => (v1.param_kinds.clone(), v1.return_kinds.clone()),
                        None => load_signature_v1(&storage, &address, &module_name, &function_name),
                    };
                    assert_eq!(
                        param_kinds.len(),
                        args.len(),
                        "function requires a different number of arguments"
                    );
                    let (v2_output, v2_gc_count) = execute_function_v2(
                        &guard,
                        &module_provider,
                        &address,
                        &module_name,
                        &function_name,
                        &args,
                        &param_kinds,
                        &return_kinds,
                        heap_size,
                    );
                    (v2_output.display, v2_gc_count)
                } else {
                    (String::new(), 0)
                };

                let v1_display = v1.map(|v| v.output.display).unwrap_or_default();
                check_output(&checks, &v1_display, &v2_display, v2_gc_count)?;
            },
        }
    }

    if !snapshot.is_empty() {
        let baseline = test_path.with_extension("exp");
        move_prover_test_utils::baseline_test::verify_or_update_baseline(&baseline, &snapshot)?;
    }

    Ok(())
}

/// Native table for the legacy VM. This includes both the real Aptos production
/// natives and some toy ones for tests.
fn v1_native_table() -> NativeFunctionTable {
    let mut table = aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    );
    table.extend(crate::v1_test_natives::make_all_v1_test_natives());
    table
}

/// The compiled Move stdlib, compiled once and shared across tests.
fn stdlib_modules() -> &'static [CompiledModule] {
    static STDLIB: OnceLock<Vec<CompiledModule>> = OnceLock::new();
    STDLIB.get_or_init(|| compile_move_stdlib().expect("Move stdlib compiles"))
}

fn test_utils_modules() -> &'static [CompiledModule] {
    static TEST_UTILS: OnceLock<Vec<CompiledModule>> = OnceLock::new();
    TEST_UTILS.get_or_init(|| {
        compile_move_path(Path::new(crate::compile::TEST_UTILS_PATH))
            .expect("test_utils library compiles")
    })
}

/// Output of V1 execution, plus the parameter and return types so V2 can
/// place args and read its result region with matching widths.
struct V1Output {
    output: Output,
    param_kinds: Vec<PrimitiveKind>,
    return_kinds: Vec<PrimitiveKind>,
}

/// Maps a function's parameter and return types to [`PrimitiveKind`]s. Panics on
/// any non-primitive type — the harness only supports primitive args/returns.
fn primitive_kinds(
    param_tys: &[Type],
    return_tys: &[Type],
    env: &RuntimeEnvironment,
) -> (Vec<PrimitiveKind>, Vec<PrimitiveKind>) {
    let param_kinds = param_tys
        .iter()
        .map(|ty| {
            PrimitiveKind::from_type(ty).expect("Only primitive argument types are supported")
        })
        .collect::<Vec<_>>();
    let return_kinds = return_tys
        .iter()
        .map(|ty| PrimitiveKind::from_return_type(ty, env))
        .collect::<Vec<_>>();
    (param_kinds, return_kinds)
}

/// Loads a function via the legacy VM and returns its parameter and return
/// kinds, without executing it. Used to size the V2 arg/return regions when the
/// legacy VM is skipped for a step (no `CHECK-V1`/shared checks).
fn load_signature_v1(
    storage: &InMemoryStorage,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
) -> (Vec<PrimitiveKind>, Vec<PrimitiveKind>) {
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
        &[],
    ) {
        Ok(function) => function,
        Err(err) => panic!("Failed to load function: {}", err),
    };

    primitive_kinds(
        function.param_tys(),
        function.return_tys(),
        module_storage.runtime_environment(),
    )
}

/// Execute a function via legacy MoveVM and returns normalized output.
fn execute_function_v1(
    storage: &InMemoryStorage,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    args: &[String],
) -> V1Output {
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
        // TODO(completeness): support type arguments.
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
    let (param_kinds, return_kinds) = primitive_kinds(
        function.param_tys(),
        function.return_tys(),
        module_storage.runtime_environment(),
    );
    let serialized_args = param_kinds
        .iter()
        .zip(args.iter())
        .map(|(kind, arg)| kind.to_move_value(arg).simple_serialize().unwrap())
        .collect::<Vec<_>>();

    // Seed the native extensions with the same fixed inputs as the mono-move
    // side, so extension-backed natives produce matching output.
    let state_storage_view = MockEmptyStateStorage {
        usage: StateStorageUsage::new(TEST_STATE_ITEMS as usize, TEST_STATE_BYTES as usize),
    };
    let mut extensions = NativeContextExtensions::default();
    let user_transaction_context = UserTransactionContext::new(
        AccountAddress::ZERO,
        vec![],
        AccountAddress::ZERO,
        0,
        0,
        0,
        None,
        None,
        TransactionIndexKind::BlockExecution {
            transaction_index: TEST_TXN_INDEX,
        },
        false,
    );
    extensions.add(NativeTransactionContext::new(
        TEST_TXN_HASH.to_vec(),
        vec![],
        0,
        Some(user_transaction_context),
        TEST_SESSION_COUNTER,
    ));
    extensions.add(NativeObjectContext::default());
    extensions.add(NativeStateStorageContext::new(&state_storage_view));
    extensions.add(NativeEventContext::default());

    let mut data_cache = TransactionDataCache::empty();
    let output = match MoveVM::execute_loaded_function(
        function,
        serialized_args,
        &mut MoveVmDataCacheAdapter::new(&mut data_cache, storage, &loader),
        &mut gas_meter,
        &mut traversal_context,
        &mut extensions,
        &loader,
    ) {
        Ok(result) => {
            let vals = result
                .return_values
                .iter()
                .zip(return_kinds.iter())
                .map(|((bytes, _layout), kind)| match kind {
                    PrimitiveKind::Utf8String => {
                        render_utf8(&bcs::from_bytes::<Vec<u8>>(bytes).expect("BCS vector<u8>"))
                    },
                    PrimitiveKind::ByteVector => {
                        render_bytes(&bcs::from_bytes::<Vec<u8>>(bytes).expect("BCS vector<u8>"))
                    },
                    _ => kind.format_bytes(bytes),
                })
                .collect::<Vec<_>>();
            let events = finalize_events_v1(&extensions);
            Output {
                display: render_execution_output(&vals, &events),
            }
        },
        Err(err) if err.major_status() == StatusCode::ABORTED => {
            let code = err.sub_status().unwrap();
            let display = match err.message() {
                Some(m) => format!("aborted: code {} ({})", code, m),
                None => format!("aborted: code {}", code),
            };
            Output { display }
        },
        Err(err) => Output {
            display: format!("error: {}", err),
        },
    };
    V1Output {
        output,
        param_kinds,
        return_kinds,
    }
}

/// Executes a function via MonoMove VM. Returns the normalized output and the
/// number of garbage collections that ran (for `CHECK-GC-COUNT`).
fn execute_function_v2(
    guard: &ExecutionGuard<'_>,
    module_provider: &InMemoryModuleProvider,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    args: &[String],
    arg_kinds: &[PrimitiveKind],
    return_kinds: &[PrimitiveKind],
    heap_size: Option<usize>,
) -> (Output, usize) {
    // Seed extensions with the same fixed inputs as the legacy side.
    let extensions = seed_extensions();

    // Run through the shared pipeline engine. Argument placement and result
    // reading mirror mono-move's frame slot layout.
    let mut gc_count = 0;
    let outcome = crate::engine::with_mono_function(
        guard,
        module_provider,
        *address,
        module_name,
        function_name,
        extensions,
        heap_size,
        |runner| {
            let result = runner.run(
                |interpreter| {
                    let mut offset: u32 = 0;
                    for (arg, kind) in args.iter().zip(arg_kinds.iter()) {
                        offset = mono_move_core::align_up_u32(offset, kind.align());
                        let bytes = kind.parse_to_bytes(arg);
                        interpreter.set_root_arg(offset, &bytes);
                        offset += kind.size();
                    }
                },
                |interpreter| {
                    let mut ret_off: u32 = 0;
                    let mut vals = Vec::with_capacity(return_kinds.len());
                    for kind in return_kinds {
                        ret_off = mono_move_core::align_up_u32(ret_off, kind.align());
                        match kind {
                            PrimitiveKind::Utf8String => {
                                let content = interpreter.root_result_byte_vector_for_test(ret_off);
                                vals.push(render_utf8(&content));
                            },
                            PrimitiveKind::ByteVector => {
                                let content = interpreter.root_result_byte_vector_for_test(ret_off);
                                vals.push(render_bytes(&content));
                            },
                            _ => {
                                let bytes =
                                    interpreter.root_result_bytes_for_test(ret_off, kind.size());
                                vals.push(kind.format_bytes(bytes));
                            },
                        }
                        ret_off += kind.size();
                    }
                    // SAFETY: the heap (and every pointer the events embed) is
                    // live while the interpreter is.
                    let events = unsafe { finalize_events_v2(interpreter.extensions(), guard) };
                    (vals, events)
                },
            );
            gc_count = runner.gc_count();
            result
        },
    );

    let display = match outcome {
        Err(err) => format!("error: {}", err),
        Ok(RunResult::Error(err)) => format!("error: {}", err),
        Ok(RunResult::Aborted { code, message }) => match message {
            Some(m) => format!("aborted: code {} ({})", code, m),
            None => format!("aborted: code {}", code),
        },
        Ok(RunResult::Success((vals, events))) => render_execution_output(&vals, &events),
    };
    (Output { display }, gc_count)
}

/// Kind supported as an argument or return value in differential tests
/// (the integer types plus `bool` and `address`). Mirrors mono-move's frame
/// slot layout so the same byte buffer can be used for both BCS (V1) and raw
/// frame storage (V2).
#[derive(Copy, Clone, Debug)]
enum PrimitiveKind {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    U256,
    I8,
    I16,
    I32,
    I64,
    I128,
    I256,
    Address,
    Signer,
    /// A `String` return value (return-only). Rendered as a UTF-8 string by
    /// reading the heap `vector<u8>` (V2) or decoding the BCS bytes (V1).
    Utf8String,
    /// A `vector<u8>` return value (return-only). Rendered as a `0x…` hex dump
    /// of its bytes, read from the heap (V2) or the BCS return (V1).
    ByteVector,
}

impl PrimitiveKind {
    fn from_type(ty: &Type) -> Option<Self> {
        Some(match ty {
            Type::Bool => PrimitiveKind::Bool,
            Type::U8 => PrimitiveKind::U8,
            Type::U16 => PrimitiveKind::U16,
            Type::U32 => PrimitiveKind::U32,
            Type::U64 => PrimitiveKind::U64,
            Type::U128 => PrimitiveKind::U128,
            Type::U256 => PrimitiveKind::U256,
            Type::I8 => PrimitiveKind::I8,
            Type::I16 => PrimitiveKind::I16,
            Type::I32 => PrimitiveKind::I32,
            Type::I64 => PrimitiveKind::I64,
            Type::I128 => PrimitiveKind::I128,
            Type::I256 => PrimitiveKind::I256,
            Type::Address => PrimitiveKind::Address,
            Type::Signer => PrimitiveKind::Signer,
            _ => return None,
        })
    }

    /// Like [`Self::from_type`] but also recognizes a `0x1::string::String`
    /// return as [`PrimitiveKind::Utf8String`]. Used for return values, which
    /// (unlike arguments) may be a `String`.
    fn from_return_type(ty: &Type, env: &RuntimeEnvironment) -> Self {
        if let Some(kind) = Self::from_type(ty) {
            return kind;
        }
        // `vector<u8>` renders as a hex byte dump (distinct from `String`).
        if let Type::Vector(elem) = ty
            && matches!(&**elem, Type::U8)
        {
            return PrimitiveKind::ByteVector;
        }
        if let Ok(TypeTag::Struct(s)) = env.ty_to_ty_tag(ty)
            && s.address == AccountAddress::ONE
            && s.module.as_str() == "string"
            && s.name.as_str() == "String"
        {
            return PrimitiveKind::Utf8String;
        }
        panic!("Only primitive, vector<u8>, and String return types are supported");
    }

    fn size(self) -> u32 {
        match self {
            PrimitiveKind::Bool | PrimitiveKind::U8 | PrimitiveKind::I8 => 1,
            PrimitiveKind::U16 | PrimitiveKind::I16 => 2,
            PrimitiveKind::U32 | PrimitiveKind::I32 => 4,
            PrimitiveKind::U64
            | PrimitiveKind::I64
            | PrimitiveKind::Utf8String
            | PrimitiveKind::ByteVector => 8,
            PrimitiveKind::U128 | PrimitiveKind::I128 => 16,
            PrimitiveKind::U256
            | PrimitiveKind::I256
            | PrimitiveKind::Address
            | PrimitiveKind::Signer => 32,
        }
    }

    fn align(self) -> u32 {
        match self {
            PrimitiveKind::Bool | PrimitiveKind::U8 | PrimitiveKind::I8 => 1,
            PrimitiveKind::U16 | PrimitiveKind::I16 => 2,
            PrimitiveKind::U32 | PrimitiveKind::I32 => 4,
            PrimitiveKind::U64
            | PrimitiveKind::I64
            | PrimitiveKind::Utf8String
            | PrimitiveKind::ByteVector => 8,
            // Wide integers and addresses are 8-byte aligned in the
            // frame even though their size is larger.
            PrimitiveKind::U128
            | PrimitiveKind::I128
            | PrimitiveKind::U256
            | PrimitiveKind::I256
            | PrimitiveKind::Address
            | PrimitiveKind::Signer => 8,
        }
    }

    fn to_move_value(self, s: &str) -> MoveValue {
        match self {
            PrimitiveKind::Bool => MoveValue::Bool(parse_bool_arg(s)),
            PrimitiveKind::U8 => MoveValue::U8(s.parse().expect("invalid u8 literal")),
            PrimitiveKind::U16 => MoveValue::U16(s.parse().expect("invalid u16 literal")),
            PrimitiveKind::U32 => MoveValue::U32(s.parse().expect("invalid u32 literal")),
            PrimitiveKind::U64 => MoveValue::U64(s.parse().expect("invalid u64 literal")),
            PrimitiveKind::U128 => MoveValue::U128(s.parse().expect("invalid u128 literal")),
            PrimitiveKind::U256 => MoveValue::U256(s.parse().expect("invalid u256 literal")),
            PrimitiveKind::I8 => MoveValue::I8(s.parse().expect("invalid i8 literal")),
            PrimitiveKind::I16 => MoveValue::I16(s.parse().expect("invalid i16 literal")),
            PrimitiveKind::I32 => MoveValue::I32(s.parse().expect("invalid i32 literal")),
            PrimitiveKind::I64 => MoveValue::I64(s.parse().expect("invalid i64 literal")),
            PrimitiveKind::I128 => MoveValue::I128(s.parse().expect("invalid i128 literal")),
            PrimitiveKind::I256 => MoveValue::I256(s.parse().expect("invalid i256 literal")),
            PrimitiveKind::Address => {
                let addr = AccountAddress::from_hex_literal(s).expect("invalid address literal");
                MoveValue::Address(addr)
            },
            PrimitiveKind::Signer => {
                let addr = AccountAddress::from_hex_literal(s).expect("invalid signer literal");
                MoveValue::Signer(addr)
            },
            PrimitiveKind::Utf8String | PrimitiveKind::ByteVector => {
                unreachable!("String / vector<u8> are return-only kinds")
            },
        }
    }

    /// Parse `s` into the raw little-endian byte representation that
    /// mono-move stores in a frame slot.
    fn parse_to_bytes(self, s: &str) -> Vec<u8> {
        match self {
            PrimitiveKind::Bool => vec![parse_bool_arg(s) as u8],
            PrimitiveKind::U8 => vec![s.parse::<u8>().expect("invalid u8 literal")],
            PrimitiveKind::U16 => s
                .parse::<u16>()
                .expect("invalid u16 literal")
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::U32 => s
                .parse::<u32>()
                .expect("invalid u32 literal")
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::U64 => s
                .parse::<u64>()
                .expect("invalid u64 literal")
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::U128 => s
                .parse::<u128>()
                .expect("invalid u128 literal")
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::U256 => s
                .parse::<U256>()
                .expect("invalid u256 literal")
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::I8 => (s.parse::<i8>().expect("invalid i8 literal") as u8)
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::I16 => s
                .parse::<i16>()
                .expect("invalid i16 literal")
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::I32 => s
                .parse::<i32>()
                .expect("invalid i32 literal")
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::I64 => s
                .parse::<i64>()
                .expect("invalid i64 literal")
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::I128 => s
                .parse::<i128>()
                .expect("invalid i128 literal")
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::I256 => s
                .parse::<I256>()
                .expect("invalid i256 literal")
                .to_le_bytes()
                .to_vec(),
            PrimitiveKind::Address | PrimitiveKind::Signer => AccountAddress::from_hex_literal(s)
                .expect("invalid address literal")
                .into_bytes()
                .to_vec(),
            PrimitiveKind::Utf8String | PrimitiveKind::ByteVector => {
                unreachable!("String / vector<u8> are return-only kinds")
            },
        }
    }

    /// Format `bytes` (in the same layout produced by `parse_to_bytes`) as a
    /// decimal string (or hex for addresses).
    fn format_bytes(self, bytes: &[u8]) -> String {
        match self {
            PrimitiveKind::Bool => (bytes[0] != 0).to_string(),
            PrimitiveKind::U8 => bytes[0].to_string(),
            PrimitiveKind::U16 => u16::from_le_bytes(bytes[..2].try_into().unwrap()).to_string(),
            PrimitiveKind::U32 => u32::from_le_bytes(bytes[..4].try_into().unwrap()).to_string(),
            PrimitiveKind::U64 => u64::from_le_bytes(bytes[..8].try_into().unwrap()).to_string(),
            PrimitiveKind::U128 => u128::from_le_bytes(bytes[..16].try_into().unwrap()).to_string(),
            PrimitiveKind::U256 => U256::from_le_bytes(bytes[..32].try_into().unwrap()).to_string(),
            PrimitiveKind::I8 => (bytes[0] as i8).to_string(),
            PrimitiveKind::I16 => i16::from_le_bytes(bytes[..2].try_into().unwrap()).to_string(),
            PrimitiveKind::I32 => i32::from_le_bytes(bytes[..4].try_into().unwrap()).to_string(),
            PrimitiveKind::I64 => i64::from_le_bytes(bytes[..8].try_into().unwrap()).to_string(),
            PrimitiveKind::I128 => i128::from_le_bytes(bytes[..16].try_into().unwrap()).to_string(),
            PrimitiveKind::I256 => I256::from_le_bytes(bytes[..32].try_into().unwrap()).to_string(),
            PrimitiveKind::Address | PrimitiveKind::Signer => {
                let arr: [u8; AccountAddress::LENGTH] = bytes[..32].try_into().unwrap();
                AccountAddress::new(arr).to_hex_literal()
            },
            PrimitiveKind::Utf8String | PrimitiveKind::ByteVector => {
                unreachable!(
                    "String / vector<u8> returns are rendered from the heap, not format_bytes"
                )
            },
        }
    }
}

/// Renders raw UTF-8 bytes as a quoted string for cross-VM comparison.
fn render_utf8(bytes: &[u8]) -> String {
    format!("{:?}", String::from_utf8_lossy(bytes))
}

/// Renders raw bytes as a `0x…` hex string for cross-VM comparison.
fn render_bytes(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    let mut s = String::with_capacity(2 + bytes.len() * 2);
    s.push_str("0x");
    for b in bytes {
        write!(s, "{b:02x}").unwrap();
    }
    s
}

/// Parse a boolean argument literal. Only `true`/`false` are accepted; the
/// integer kinds parse decimal, so a clear error guards against passing a
/// bool as `0`/`1`.
fn parse_bool_arg(s: &str) -> bool {
    match s {
        "true" => true,
        "false" => false,
        other => panic!("bool args must be `true` or `false`, got {:?}", other),
    }
}
