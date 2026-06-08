// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Executes parsed test steps against both MoveVM and mono-move, producing
//! normalized output for comparison.

use crate::{
    compile::{compile, compile_move_stdlib, SourceKind},
    matcher::check_output,
    module_provider::InMemoryModuleProvider,
    parser::{PrintSection, Step},
    print_sections,
};
use anyhow::{anyhow, bail};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_types::on_chain_config::{Features, TimedFeaturesBuilder};
use aptos_vm::natives::aptos_natives;
use mono_move_core::{
    native::{NativeName, ProductionContextFamily, ProductionNativeRegistry},
    types::EMPTY_TYPE_LIST,
    Interner,
};
use mono_move_gas::SimpleGasMeter;
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_natives::{make_all_production_natives, make_all_test_natives};
use mono_move_runtime::{ExecutionContext, InterpreterContext, RuntimeStatus, TransactionContext};
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress,
    identifier::IdentStr,
    int256::{I256, U256},
    language_storage::ModuleId,
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
    RuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{gas::UnmeteredGasMeter, loaded_data::runtime_types::Type};
use std::{path::Path, sync::OnceLock};

/// Execution output from a VM as a normalized display string.
struct Output {
    display: String,
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
    for module in stdlib_modules() {
        let mut blob = vec![];
        module
            .serialize(&mut blob)
            .expect("stdlib module serializes");
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
                checks,
            } => {
                let v1_output =
                    execute_function_v1(&storage, &address, &module_name, &function_name, &args);
                let v2_output = execute_function_v2(
                    &guard,
                    &module_provider,
                    &address,
                    &module_name,
                    &function_name,
                    &args,
                    &v1_output.param_kinds,
                    &v1_output.return_kinds,
                );
                check_output(&checks, &v1_output.output.display, &v2_output.display)?;
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

/// Output of V1 execution, plus the parameter and return types so V2 can
/// place args and read its result region with matching widths.
struct V1Output {
    output: Output,
    param_kinds: Vec<PrimitiveKind>,
    return_kinds: Vec<PrimitiveKind>,
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
    let param_kinds = function
        .param_tys()
        .iter()
        .map(|ty| {
            PrimitiveKind::from_type(ty).expect("Only primitive argument types are supported")
        })
        .collect::<Vec<_>>();
    let return_kinds = function
        .return_tys()
        .iter()
        .map(|ty| PrimitiveKind::from_type(ty).expect("Only primitive return types are supported"))
        .collect::<Vec<_>>();
    let serialized_args = param_kinds
        .iter()
        .zip(args.iter())
        .map(|(kind, arg)| kind.to_move_value(arg).simple_serialize().unwrap())
        .collect::<Vec<_>>();

    let mut data_cache = TransactionDataCache::empty();
    let output = match MoveVM::execute_loaded_function(
        function,
        serialized_args,
        &mut MoveVmDataCacheAdapter::new(&mut data_cache, storage, &loader),
        &mut gas_meter,
        &mut traversal_context,
        &mut NativeContextExtensions::default(),
        &loader,
    ) {
        Ok(result) => {
            let vals = result
                .return_values
                .iter()
                .zip(return_kinds.iter())
                .map(|((bytes, _layout), kind)| kind.format_bytes(bytes))
                .collect::<Vec<_>>();
            Output {
                display: format!("results: {}", vals.join(", ")),
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

/// Executes a function via MonoMove VM, and returns normalized output.
fn execute_function_v2(
    guard: &ExecutionGuard<'_>,
    module_provider: &InMemoryModuleProvider,
    address: &AccountAddress,
    module_name: &IdentStr,
    function_name: &IdentStr,
    args: &[String],
    arg_kinds: &[PrimitiveKind],
    return_kinds: &[PrimitiveKind],
) -> Output {
    // Construct a per-transaction context.
    let mut natives = ProductionNativeRegistry::<SimpleGasMeter>::new();
    natives
        .register_all(
            make_all_test_natives::<ProductionContextFamily<SimpleGasMeter>>()
                .into_iter()
                .chain(make_all_production_natives::<
                    ProductionContextFamily<SimpleGasMeter>,
                >())
                .map(|(addr, module, function, func)| {
                    let name = NativeName {
                        module: guard.module_id_of(&addr, &module),
                        function: guard.identifier_of(&function),
                    };
                    (name, func)
                }),
        )
        .expect("natives have unique qualified names");
    let loader = Loader::new_with_policy(
        guard,
        module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &natives,
    );
    let mut txn_ctx = TransactionContext::new(
        loader,
        SimpleGasMeter::new(u64::MAX),
        &mono_move_core::NO_RESOURCE_PROVIDER,
        &natives,
    );

    // Resolve the entry function via load_function so the entry module is
    // lazily loaded into the read-set and gas is charged for the load.
    let id = guard
        .intern_address_name(address, module_name)
        .into_global_arena_ptr();
    let function_name = guard
        .intern_identifier(function_name)
        .into_global_arena_ptr();

    // SAFETY: the pointer lives in a `LoadedModule`'s arena. While `guard`
    // is held, the global executable cache cannot enter the maintenance
    // phase, so no arena reset can happen for the duration of this step.
    let function = match txn_ctx.load_function(id, function_name, EMPTY_TYPE_LIST) {
        Ok(p) => unsafe { p.as_ref_unchecked() },
        Err(err) => {
            return Output {
                display: format!("error: {}", err),
            };
        },
    };

    let mut interpreter = InterpreterContext::new(&mut txn_ctx, function);

    let mut offset: u32 = 0;
    for (arg, kind) in args.iter().zip(arg_kinds.iter()) {
        offset = align_up(offset, kind.align());
        let bytes = kind.parse_to_bytes(arg);
        interpreter.set_root_arg(offset, &bytes);
        offset += kind.size();
    }

    match interpreter.run() {
        Err(err) => Output {
            display: format!("error: {}", err),
        },
        Ok(RuntimeStatus::Aborted { code, message }) => {
            let display = match message {
                Some(m) => format!("aborted: code {} ({})", code, m),
                None => format!("aborted: code {}", code),
            };
            Output { display }
        },
        Ok(RuntimeStatus::Success) => {
            let mut ret_off: u32 = 0;
            let mut vals = Vec::with_capacity(return_kinds.len());
            for kind in return_kinds {
                ret_off = align_up(ret_off, kind.align());
                let bytes = interpreter.root_result_bytes(ret_off, kind.size());
                vals.push(kind.format_bytes(bytes));
                ret_off += kind.size();
            }
            Output {
                display: format!("results: {}", vals.join(", ")),
            }
        },
    }
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
            _ => return None,
        })
    }

    fn size(self) -> u32 {
        match self {
            PrimitiveKind::Bool | PrimitiveKind::U8 | PrimitiveKind::I8 => 1,
            PrimitiveKind::U16 | PrimitiveKind::I16 => 2,
            PrimitiveKind::U32 | PrimitiveKind::I32 => 4,
            PrimitiveKind::U64 | PrimitiveKind::I64 => 8,
            PrimitiveKind::U128 | PrimitiveKind::I128 => 16,
            PrimitiveKind::U256 | PrimitiveKind::I256 | PrimitiveKind::Address => 32,
        }
    }

    fn align(self) -> u32 {
        match self {
            PrimitiveKind::Bool | PrimitiveKind::U8 | PrimitiveKind::I8 => 1,
            PrimitiveKind::U16 | PrimitiveKind::I16 => 2,
            PrimitiveKind::U32 | PrimitiveKind::I32 => 4,
            PrimitiveKind::U64 | PrimitiveKind::I64 => 8,
            // Wide integers and addresses are 8-byte aligned in the
            // frame even though their size is larger.
            PrimitiveKind::U128
            | PrimitiveKind::I128
            | PrimitiveKind::U256
            | PrimitiveKind::I256
            | PrimitiveKind::Address => 8,
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
            PrimitiveKind::Address => AccountAddress::from_hex_literal(s)
                .expect("invalid address literal")
                .into_bytes()
                .to_vec(),
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
            PrimitiveKind::Address => {
                let arr: [u8; AccountAddress::LENGTH] = bytes[..32].try_into().unwrap();
                AccountAddress::new(arr).to_hex_literal()
            },
        }
    }
}

fn align_up(offset: u32, align: u32) -> u32 {
    (offset + align - 1) & !(align - 1)
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
