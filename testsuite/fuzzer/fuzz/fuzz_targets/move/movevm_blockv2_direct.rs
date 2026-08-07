#![no_main]

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use libfuzzer_sys::{fuzz_target, Corpus};
use move_binary_format::{
    compatibility::Compatibility,
    deserializer::DeserializerConfig,
    errors::{PartialVMError, VMError},
    file_format::{CompiledModule, CompiledScript, FunctionDefinitionIndex},
    file_format_common::{IDENTIFIER_SIZE_MAX, VERSION_MAX},
};
use move_bytecode_verifier::VerifierConfig;
use move_core_types::{
    account_address::AccountAddress,
    gas_algebra::GasQuantity,
    identifier::Identifier,
    language_storage::TypeTag,
    transaction_argument::{convert_txn_args, TransactionArgument},
    value::MoveValue,
    vm_status::{StatusCode, StatusType},
};
use move_vm_runtime::{
    config::VMConfig,
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::MoveVM,
    native_extensions::NativeContextExtensions,
    AsUnsyncCodeStorage, InstantiatedFunctionLoader, LegacyLoaderConfig, RuntimeEnvironment,
    ScriptLoader, StagingModuleStorage,
};
use move_vm_test_utils::{
    gas_schedule::{CostTable, GasCost, GasStatus},
    InMemoryStorage,
};
use move_vm_types::loaded_data::runtime_types::Type;
use once_cell::sync::Lazy;
use std::env;

mod blockv2_fuzz_config;
mod utils;

use blockv2_fuzz_config::{apply_verifier_config_overrides, apply_vm_config_overrides, env_u64};
use fuzzer::{BlockExecVariantV2, RunnableBlockStateV2, RunnableBlockTransactionV2};
use utils::vm::{
    filter_bad_modules, filter_bad_tx, group_modules_by_address_topo, has_invalid_split_blocks,
    module_self_id, module_self_id_or_keep, normalize_module_for_fuzz, resolve_function_name,
    resolve_module_ref, resolve_module_refs, serialize_module_for_version, verify_module_fast,
    verify_script_fast,
};

const BYTECODE_VERSION: u32 = VERSION_MAX;
const MAX_BLOCK_MODULES: usize = 32;
const MAX_BLOCK_TXNS: usize = 24;
const MAX_TYPE_ARGS: usize = 8;
const MAX_ARGS: usize = 16;
const MAX_SIGNER_ARGS: usize = 16;
const DEFAULT_EXECUTION_GAS: u64 = 2_000;

static VERIFIER_CONFIG: Lazy<VerifierConfig> = Lazy::new(|| {
    let mut config = VerifierConfig::production();
    config.enable_resource_access_control = false;
    apply_verifier_config_overrides(&mut config);
    config
});
static DESERIALIZER_CONFIG: Lazy<DeserializerConfig> =
    Lazy::new(|| DeserializerConfig::new(BYTECODE_VERSION, IDENTIFIER_SIZE_MAX));
static EXECUTION_GAS: Lazy<u64> =
    Lazy::new(|| env_u64("APTOS_MOVEVM_DIRECT_FUZZ_GAS", DEFAULT_EXECUTION_GAS));

fn direct_vm_config() -> VMConfig {
    let mut config = VMConfig {
        verifier_config: VERIFIER_CONFIG.clone(),
        ..VMConfig::default_for_test()
    };
    apply_vm_config_overrides(&mut config);
    config
}

fn new_storage() -> InMemoryStorage {
    InMemoryStorage::new_with_runtime_environment(RuntimeEnvironment::new_with_config(
        vec![],
        direct_vm_config(),
    ))
}

fn log_interesting_status(source: &str, status_code: StatusCode) {
    let status_name = format!("{status_code:?}");
    let kind = match status_code.status_type() {
        StatusType::InvariantViolation => Some("invariant violation"),
        _ if status_name.contains("TYPE") => Some("type-named error"),
        _ => None,
    };

    if let Some(kind) = kind {
        eprintln!("movevm_blockv2_direct: {kind} via {source}: status_code = {status_code:?}",);
    }
}

fn should_debug_status(status_code: StatusCode) -> bool {
    env::var("APTOS_MOVEVM_DIRECT_DEBUG_STATUS")
        .ok()
        .is_some_and(|statuses| {
            statuses
                .split(',')
                .any(|status| status.trim() == format!("{status_code:?}"))
        })
}

fn keep_vm_error(source: &'static str, error: VMError) -> Corpus {
    tdbg!("vm error", source, &error);
    log_interesting_status(source, error.major_status());
    if should_debug_status(error.major_status()) {
        eprintln!(
            "movevm_blockv2_direct debug via {source}: {}",
            error.format_test_output(true)
        );
        if env::var_os("APTOS_MOVEVM_DIRECT_DEBUG_NO_PANIC").is_none() {
            panic!(
                "movevm_blockv2_direct debug status {:?}",
                error.major_status()
            );
        }
    }
    Corpus::Keep
}

fn keep_partial_vm_error(source: &'static str, error: PartialVMError) -> Corpus {
    tdbg!("partial vm error", source, &error);
    log_interesting_status(source, error.major_status());
    if should_debug_status(error.major_status()) {
        eprintln!("movevm_blockv2_direct debug via {source}: {error}");
        if env::var_os("APTOS_MOVEVM_DIRECT_DEBUG_NO_PANIC").is_none() {
            panic!(
                "movevm_blockv2_direct debug status {:?}",
                error.major_status()
            );
        }
    }
    Corpus::Keep
}

fn metered_gas_status() -> GasStatus {
    GasStatus::new(
        CostTable {
            instruction_table: vec![GasCost::new(1, 1); 255],
        },
        GasQuantity::new(*EXECUTION_GAS),
    )
}

fn verify_module(module: CompiledModule) -> Result<CompiledModule, Corpus> {
    tdbg!("verify module", module_self_id(&module));
    let module = normalize_module_for_fuzz(module)?;
    verify_module_fast(&module, &VERIFIER_CONFIG, &DESERIALIZER_CONFIG)?;
    Ok(module)
}

fn publish_package(
    storage: &mut InMemoryStorage,
    modules: Vec<CompiledModule>,
) -> Result<(), Corpus> {
    tdbg!("publish package", modules.len());
    if modules.is_empty() {
        return Err(Corpus::Reject);
    }

    let modules = modules
        .into_iter()
        .map(verify_module)
        .collect::<Result<Vec<_>, _>>()?;
    let sender = *module_self_id_or_keep(modules.first().ok_or(Corpus::Reject)?)?.address();
    tdbg!("publish package sender", sender);
    for module in &modules {
        if module_self_id_or_keep(module)?.address() != &sender {
            return Err(Corpus::Reject);
        }
    }

    let module_bytes = modules
        .iter()
        .map(|module| serialize_module_for_version(module, VERSION_MAX))
        .collect::<Result<Vec<_>, _>>()?;
    let verified_bundle = {
        let module_storage = storage.as_unsync_code_storage();
        StagingModuleStorage::create_with_compat_config(
            &sender,
            Compatibility::no_check(),
            &module_storage,
            module_bytes.into_iter().map(Into::into).collect(),
        )
        .map_err(|error| keep_vm_error("publish_package_create_staging_storage", error))?
        .release_verified_module_bundle()
    };

    for (module_id, bytes) in verified_bundle {
        storage.add_module_bytes(module_id.address(), module_id.name(), bytes);
    }
    Ok(())
}

fn publish_dependency_modules(
    storage: &mut InMemoryStorage,
    dep_modules: &[CompiledModule],
) -> Result<(), Corpus> {
    tdbg!("publish dependency modules", dep_modules.len());
    let dep_modules = dep_modules
        .iter()
        .cloned()
        .map(verify_module)
        .collect::<Result<Vec<_>, _>>()?;
    let packages = group_modules_by_address_topo(dep_modules)?;
    tdbg!("dependency package count", packages.len());
    for package in packages {
        publish_package(storage, package)?;
    }
    Ok(())
}

fn serialize_move_args(args: Vec<MoveValue>) -> Result<Vec<Vec<u8>>, Corpus> {
    let args = args
        .into_iter()
        .take(MAX_ARGS)
        .map(TransactionArgument::try_from)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| Corpus::Keep)?;
    Ok(convert_txn_args(&args))
}

fn signer_args(function_param_tys: &[Type]) -> Result<Vec<Vec<u8>>, Corpus> {
    let signer_count = function_param_tys
        .iter()
        .filter(|ty| matches!(ty, Type::Signer) || ty.paranoid_check_is_signer_ref_ty().is_ok())
        .count();
    if signer_count > MAX_SIGNER_ARGS {
        return Err(Corpus::Keep);
    }

    Ok(vec![
        MoveValue::Signer(AccountAddress::TWO)
            .simple_serialize()
            .ok_or(Corpus::Keep)?;
        signer_count
    ])
}

fn apply_effects(
    storage: &mut InMemoryStorage,
    data_cache: TransactionDataCache,
) -> Result<(), Corpus> {
    tdbg!("apply effects");
    let code_storage = storage.as_unsync_code_storage();
    let change_set = data_cache
        .into_effects(&code_storage)
        .map_err(|error| keep_partial_vm_error("data_cache_into_effects", error))?;
    drop(code_storage);
    storage
        .apply(change_set)
        .map_err(|error| keep_partial_vm_error("storage_apply_effects", error))
}

fn execute_script(
    storage: &mut InMemoryStorage,
    mut script: CompiledScript,
    mut type_args: Vec<TypeTag>,
    args: Vec<MoveValue>,
) -> Result<(), Corpus> {
    tdbg!("execute script");
    script.version = VERSION_MAX;
    type_args.truncate(MAX_TYPE_ARGS);
    let mut serialized_args = serialize_move_args(args)?;

    tdbg!("verify script");
    verify_script_fast(&script, &VERIFIER_CONFIG, &DESERIALIZER_CONFIG)?;
    let mut script_bytes = vec![];
    script
        .serialize_for_version(Some(VERSION_MAX), &mut script_bytes)
        .map_err(|_| Corpus::Keep)?;

    let code_storage = storage.as_unsync_code_storage();
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);
    let mut gas_meter = metered_gas_status();
    let mut data_cache = TransactionDataCache::empty();

    move_vm_runtime::dispatch_loader!(&code_storage, loader, {
        let function = loader
            .load_script(
                &LegacyLoaderConfig::unmetered(),
                &mut gas_meter,
                &mut traversal_context,
                &script_bytes,
                &type_args,
            )
            .map_err(|error| keep_vm_error("load_script", error))?;
        let mut all_args = signer_args(function.param_tys())?;
        all_args.append(&mut serialized_args);

        MoveVM::execute_loaded_function(
            function,
            all_args,
            &mut MoveVmDataCacheAdapter::new(&mut data_cache, storage, &loader),
            &mut gas_meter,
            &mut traversal_context,
            &mut NativeContextExtensions::default(),
            &loader,
        )
        .map_err(|error| keep_vm_error("execute_script_function", error))
    })?;

    drop(code_storage);
    apply_effects(storage, data_cache)
}

fn function_name(
    module: &CompiledModule,
    function: FunctionDefinitionIndex,
) -> Result<Identifier, Corpus> {
    resolve_function_name(module, function).map_err(|_| Corpus::Keep)
}

fn execute_function(
    storage: &mut InMemoryStorage,
    module: CompiledModule,
    function: FunctionDefinitionIndex,
    mut type_args: Vec<TypeTag>,
    mut args: Vec<Vec<u8>>,
) -> Result<(), Corpus> {
    tdbg!("execute function");
    type_args.truncate(MAX_TYPE_ARGS);
    args.truncate(MAX_ARGS);

    let module = verify_module(module)?;
    let module_id = module_self_id_or_keep(&module)?;
    let function_name = function_name(&module, function)?;

    let code_storage = storage.as_unsync_code_storage();
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);
    let mut gas_meter = metered_gas_status();
    let mut data_cache = TransactionDataCache::empty();

    move_vm_runtime::dispatch_loader!(&code_storage, loader, {
        let function = loader
            .load_instantiated_function(
                &LegacyLoaderConfig::unmetered(),
                &mut gas_meter,
                &mut traversal_context,
                &module_id,
                &function_name,
                &type_args,
            )
            .map_err(|error| keep_vm_error("load_instantiated_function", error))?;
        let mut all_args = signer_args(function.param_tys())?;
        all_args.append(&mut args);

        MoveVM::execute_loaded_function(
            function,
            all_args,
            &mut MoveVmDataCacheAdapter::new(&mut data_cache, storage, &loader),
            &mut gas_meter,
            &mut traversal_context,
            &mut NativeContextExtensions::default(),
            &loader,
        )
        .map_err(|error| keep_vm_error("execute_entry_function", error))
    })?;

    drop(code_storage);
    apply_effects(storage, data_cache)
}

fn execute_transaction(
    storage: &mut InMemoryStorage,
    modules: &[CompiledModule],
    tx: &RunnableBlockTransactionV2,
) -> Result<(), Corpus> {
    tdbg!("execute transaction", &tx.exec_variant);
    match &tx.exec_variant {
        BlockExecVariantV2::Script {
            _script,
            _type_args,
            _args,
        } => execute_script(storage, _script.clone(), _type_args.clone(), _args.clone()),
        BlockExecVariantV2::CallFunction {
            _module_idx,
            _function,
            _type_args,
            _args,
        } => {
            let module = resolve_module_ref(modules, *_module_idx)?.clone();
            execute_function(
                storage,
                module,
                *_function,
                _type_args.clone(),
                _args.clone(),
            )
        },
        BlockExecVariantV2::Publish { _module_idxs } => {
            let modules = resolve_module_refs(modules, _module_idxs)?;
            publish_package(storage, modules)
        },
        BlockExecVariantV2::SplitBlock => Ok(()),
    }
}

fn run_case(mut input: RunnableBlockStateV2) -> Result<(), Corpus> {
    tdbg!(&input);
    tdbg!("filtering fuzz case");
    if input.modules.len() > MAX_BLOCK_MODULES
        || input.transactions.is_empty()
        || input.transactions.len() > MAX_BLOCK_TXNS
    {
        return Err(Corpus::Reject);
    }

    if has_invalid_split_blocks(&input.transactions) {
        return Err(Corpus::Keep);
    }

    // fail fast
    tdbg!("checking module ids and transaction shape");
    filter_bad_modules(&mut input.modules)?;
    for tx in &input.transactions {
        filter_bad_tx(&tx.exec_variant)?;
    }

    tdbg!("collecting publish modules");
    let publish_modules = input
        .transactions
        .iter()
        .filter_map(|tx| match &tx.exec_variant {
            BlockExecVariantV2::Publish { _module_idxs } => {
                Some(resolve_module_refs(&input.modules, _module_idxs))
            },
            _ => None,
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let preload_modules = input
        .modules
        .iter()
        .filter(|module| !publish_modules.contains(module))
        .cloned()
        .collect::<Vec<_>>();
    tdbg!(
        "preload and transaction counts",
        preload_modules.len(),
        input.transactions.len()
    );
    let mut storage = new_storage();
    publish_dependency_modules(&mut storage, &preload_modules)?;
    tdbg!("executing transactions");
    for tx in &input.transactions {
        execute_transaction(&mut storage, &input.modules, tx)?;
    }

    Ok(())
}

fuzz_target!(|fuzz_data: RunnableBlockStateV2| -> Corpus {
    run_case(fuzz_data).err().unwrap_or(Corpus::Keep)
});
