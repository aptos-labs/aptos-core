// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Legacy Move VM (V1) harness: runs a transaction's entry function on the legacy Move VM and
//! returns its outcome and timing.
//!
//! Modules come from a hot cache (lazily loaded, warmed by an untimed trial run); resources are
//! read through an Aptos resolver over the read-set, with a fresh data cache per run. Paranoid type
//! checks and gas metering are off; only argument deserialization + execution are timed.

use crate::{
    compare::{ExecOutcome, FailureKind},
    data::BenchmarkInput,
    timing::{collect_samples, TimingConfig},
    BenchmarkRun,
};
use anyhow::{anyhow, Context};
use aptos_framework_natives::{
    aggregator_natives::NativeAggregatorContext,
    code::NativeCodeContext,
    cryptography::{algebra::AlgebraContext, ristretto255_point::NativeRistrettoPointContext},
    event::NativeEventContext,
    object::NativeObjectContext,
    randomness::RandomnessContext,
    state_storage::NativeStateStorageContext,
    transaction_context::NativeTransactionContext,
};
use aptos_gas_schedule::{MiscGasParameters, NativeGasParameters, LATEST_GAS_FEATURE_VERSION};
use aptos_table_natives::NativeTableContext;
use aptos_types::{
    on_chain_config::{Features, TimedFeaturesBuilder},
    transaction::user_transaction_context::UserTransactionContext,
};
use aptos_vm::{
    data_cache::AsMoveResolver, move_vm_ext::AptosMoveResolver, natives::aptos_natives,
};
use move_binary_format::errors::{VMError, VMResult};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{ModuleId, TypeTag},
    value::MoveValue,
    vm_status::{StatusCode, StatusType, VMStatus},
};
use move_vm_runtime::{
    config::VMConfig,
    data_cache::{MoveVmDataCacheAdapter, TransactionDataCache},
    dispatch_loader,
    module_traversal::{TraversalContext, TraversalStorage},
    move_vm::MoveVM,
    native_extensions::NativeContextExtensions,
    AsUnsyncModuleStorage, InstantiatedFunctionLoader, LegacyLoaderConfig, LoadedFunction, Loader,
    RuntimeEnvironment,
};
use move_vm_test_utils::InMemoryStorage;
use move_vm_types::{gas::UnmeteredGasMeter, loaded_data::runtime_types::Type};
use std::time::{Duration, Instant};

/// Runs the entry function on the legacy Move VM, returning its outcome and timing.
pub fn run(input: &BenchmarkInput, timing: &TimingConfig) -> anyhow::Result<BenchmarkRun> {
    // One-time setup. Modules come from in-memory storage; resources and native-extension data come
    // from the Aptos resolver over the read-set.
    let storage = build_storage(input).context("failed to build V1 in-memory module storage")?;
    let module_storage = storage.as_unsync_module_storage();
    let resolver = input.read_set.as_ref().as_move_resolver();

    let module_id = input.entry.module().clone();
    let function_name = input.entry.function();
    let ty_args = input.entry.ty_args().to_vec();

    dispatch_loader!(&module_storage, loader, {
        // Build the full argument vector (leading &signer args + the transaction's args). Needs the
        // loaded function to count signer parameters.
        let args = {
            let func = load(&loader, &module_id, function_name, &ty_args)
                .map_err(|e| anyhow!("failed to load entry function: {:?}", e))?;
            build_args(&func, input)?
        };

        // Trial run: determine the outcome. Also warms the hot module cache (lazily loading the
        // modules the execution touches) so the measured runs are warm.
        let outcome = trial(
            &loader,
            &resolver,
            &module_id,
            function_name,
            &ty_args,
            &args,
            &input.user_context,
            input.chain_id,
        )?;

        // Timing: measure only "deserialize args + execute" across many samples.
        let samples = collect_samples(timing, || {
            timed_once(
                &loader,
                &resolver,
                &module_id,
                function_name,
                &ty_args,
                &args,
                &input.user_context,
                input.chain_id,
            )
        });

        Ok(BenchmarkRun { outcome, samples })
    })
}

/// Builds in-memory storage holding the read-set's module bytecode.
fn build_storage(input: &BenchmarkInput) -> anyhow::Result<InMemoryStorage> {
    let natives = aptos_natives(
        LATEST_GAS_FEATURE_VERSION,
        NativeGasParameters::zeros(),
        MiscGasParameters::zeros(),
        TimedFeaturesBuilder::enable_all().build(),
        Features::default(),
    );
    let vm_config = VMConfig {
        paranoid_type_checks: false,
        ..VMConfig::default()
    };
    let env = RuntimeEnvironment::new_with_config(natives, vm_config);
    let mut storage = InMemoryStorage::new_with_runtime_environment(env);

    for (module_id, bytes) in input.read_set.modules() {
        storage.add_module_bytes(module_id.address(), module_id.name(), bytes.into());
    }
    Ok(storage)
}

/// Builds the Aptos native context extensions, backed by the read-set resolver and the
/// transaction's [`UserTransactionContext`]. The transaction hash is stubbed.
fn build_extensions<'a, R: AptosMoveResolver>(
    resolver: &'a R,
    user_context: &UserTransactionContext,
    chain_id: u8,
) -> NativeContextExtensions<'a> {
    let mut extensions = NativeContextExtensions::default();
    extensions.add(NativeTableContext::new([0u8; 32], resolver));
    extensions.add(NativeRistrettoPointContext::new());
    extensions.add(AlgebraContext::new());
    extensions.add(NativeAggregatorContext::new(
        [0u8; 32], resolver, false, resolver,
    ));
    let mut randomness = RandomnessContext::new();
    randomness.mark_unbiasable();
    extensions.add(randomness);
    extensions.add(NativeTransactionContext::new(
        vec![0u8; 32],
        vec![],
        chain_id,
        Some(user_context.clone()),
        0,
    ));
    extensions.add(NativeCodeContext::new());
    extensions.add(NativeStateStorageContext::new(resolver));
    extensions.add(NativeEventContext::default());
    extensions.add(NativeObjectContext::default());
    extensions
}

fn load<L: InstantiatedFunctionLoader>(
    loader: &L,
    module_id: &ModuleId,
    function_name: &IdentStr,
    ty_args: &[TypeTag],
) -> VMResult<LoadedFunction> {
    let mut gas_meter = UnmeteredGasMeter;
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);
    loader.load_instantiated_function(
        &LegacyLoaderConfig::unmetered(),
        &mut gas_meter,
        &mut traversal_context,
        module_id,
        function_name,
        ty_args,
    )
}

/// Prepends one serialized `&signer` argument (the sender) per leading signer parameter, followed
/// by the transaction's own (non-signer) arguments.
fn build_args(func: &LoadedFunction, input: &BenchmarkInput) -> anyhow::Result<Vec<Vec<u8>>> {
    let signer_count = func
        .param_tys()
        .iter()
        .take_while(|ty| is_signer(ty))
        .count();
    let mut args = Vec::with_capacity(signer_count + input.entry.args().len());
    for _ in 0..signer_count {
        args.push(
            MoveValue::Signer(input.sender)
                .simple_serialize()
                .ok_or_else(|| anyhow!("failed to serialize signer argument"))?,
        );
    }
    args.extend(input.entry.args().iter().cloned());
    Ok(args)
}

fn is_signer(ty: &Type) -> bool {
    matches!(ty, Type::Signer) || matches!(ty.get_ref_inner_ty(), Some(Type::Signer))
}

#[allow(clippy::too_many_arguments)]
fn trial<L: Loader + InstantiatedFunctionLoader, R: AptosMoveResolver>(
    loader: &L,
    resolver: &R,
    module_id: &ModuleId,
    function_name: &IdentStr,
    ty_args: &[TypeTag],
    args: &[Vec<u8>],
    user_context: &UserTransactionContext,
    chain_id: u8,
) -> anyhow::Result<ExecOutcome> {
    let mut data_cache = TransactionDataCache::empty();
    let mut gas_meter = UnmeteredGasMeter;
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);
    let mut extensions = build_extensions(resolver, user_context, chain_id);

    let func = load(loader, module_id, function_name, ty_args)
        .map_err(|e| anyhow!("failed to load entry function: {:?}", e))?;
    let result = MoveVM::execute_loaded_function(
        func,
        args.to_vec(),
        &mut MoveVmDataCacheAdapter::new(&mut data_cache, resolver, loader),
        &mut gas_meter,
        &mut traversal_context,
        &mut extensions,
        loader,
    );

    match result {
        Ok(_) => Ok(ExecOutcome::Success),
        Err(err) => Ok(classify_error(err)),
    }
}

/// Times a single "deserialize args + execute" region. Per-run state (the empty data cache, fresh
/// extensions/traversal, and function load) is rebuilt outside the timer.
#[allow(clippy::too_many_arguments)]
fn timed_once<L: Loader + InstantiatedFunctionLoader, R: AptosMoveResolver>(
    loader: &L,
    resolver: &R,
    module_id: &ModuleId,
    function_name: &IdentStr,
    ty_args: &[TypeTag],
    args: &[Vec<u8>],
    user_context: &UserTransactionContext,
    chain_id: u8,
) -> Duration {
    let mut data_cache = TransactionDataCache::empty();
    let mut gas_meter = UnmeteredGasMeter;
    let traversal_storage = TraversalStorage::new();
    let mut traversal_context = TraversalContext::new(&traversal_storage);
    let mut extensions = build_extensions(resolver, user_context, chain_id);
    let func = load(loader, module_id, function_name, ty_args)
        .expect("entry function was already loaded during setup");
    let call_args = args.to_vec();

    let start = Instant::now();
    let _ = MoveVM::execute_loaded_function(
        func,
        call_args,
        &mut MoveVmDataCacheAdapter::new(&mut data_cache, resolver, loader),
        &mut gas_meter,
        &mut traversal_context,
        &mut extensions,
        loader,
    );
    start.elapsed()
}

fn classify_error(err: VMError) -> ExecOutcome {
    match err.into_vm_status() {
        VMStatus::MoveAbort { code, message, .. } => ExecOutcome::Aborted { code, message },
        VMStatus::ExecutionFailure {
            status_code,
            location,
            ..
        } => ExecOutcome::Failure {
            kind: map_status_code(status_code),
            detail: format!("{:?} at {:?}", status_code, location),
        },
        VMStatus::Error {
            status_code,
            message,
            ..
        } => ExecOutcome::Failure {
            kind: map_status_code(status_code),
            detail: message.unwrap_or_else(|| format!("{:?}", status_code)),
        },
        VMStatus::Executed => ExecOutcome::Failure {
            kind: FailureKind::Other,
            detail: "unexpected Executed status on error path".to_string(),
        },
    }
}

fn map_status_code(code: StatusCode) -> FailureKind {
    match code {
        StatusCode::OUT_OF_GAS => FailureKind::OutOfGas,
        StatusCode::ARITHMETIC_ERROR => FailureKind::Arithmetic,
        StatusCode::RESOURCE_DOES_NOT_EXIST | StatusCode::MISSING_DATA => {
            FailureKind::ResourceDoesNotExist
        },
        StatusCode::RESOURCE_ALREADY_EXISTS => FailureKind::ResourceAlreadyExists,
        StatusCode::VECTOR_OPERATION_ERROR => FailureKind::VectorError,
        _ => match code.status_type() {
            StatusType::InvariantViolation => FailureKind::InvariantViolation,
            StatusType::Verification => FailureKind::TypeOrReferenceSafety,
            _ => FailureKind::Other,
        },
    }
}
