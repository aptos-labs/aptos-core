// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove (V2) harness: runs a transaction's entry function on MonoMove and returns its outcome
//! and timing.

use crate::{
    compare::{ExecOutcome, FailureKind},
    data::{BenchmarkInput, ReadSet},
    timing::{collect_samples, TimingConfig},
    BenchmarkRun,
};
use anyhow::{anyhow, bail, Context, Result};
use aptos_types::{
    access_path::Path,
    state_store::{state_key::inner::StateKeyInner, TStateView},
    transaction::user_transaction_context::TransactionIndexKind,
};
use bytes::Bytes;
use mono_move_core::{
    intern_sig_token,
    native::NativeExtensions,
    types::{
        InternedType, InternedTypeList, ADDRESS_TY, BOOL_TY, I128_TY, I16_TY, I256_TY, I32_TY,
        I64_TY, I8_TY, SIGNER_TY, U128_TY, U16_TY, U256_TY, U32_TY, U64_TY, U8_TY,
    },
    Function, GasMeter, Interner, LoaderError,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy, ModuleReadSet};
use mono_move_natives::{
    EventStore, ObjectContextExtension, StorageUsageAtEpochBoundary, TransactionContextExtension,
};
use mono_move_runtime::{InterpreterContext, RuntimeError, RuntimeStatus};
use mono_move_testsuite::{
    build_natives, finalize_events_v2, InMemoryModuleProvider, InMemoryResourceProvider,
};
use move_binary_format::{access::ModuleAccess, file_format::SignatureToken, CompiledModule};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag},
};
use std::{collections::BTreeMap, time::Instant};

/// Effectively unbounded gas budget.
const GAS_BUDGET: u64 = u64::MAX;

/// The resource arena is sized as `read-set bytes * ARENA_BYTES_PER_RESOURCE_BYTE`, with a floor of
/// `MIN_ARENA_BYTES` (the flat representation can be larger than BCS).
const MIN_ARENA_BYTES: usize = 16 * 1024 * 1024;
const ARENA_BYTES_PER_RESOURCE_BYTE: usize = 8;

/// How an entry-function parameter is filled into the root frame.
enum ParamKind {
    /// A `signer`/`&signer` parameter, filled with the sender.
    Signer { by_ref: bool },
    /// Any other parameter, deserialized from BCS into the frame.
    Value { ty: InternedType },
}

pub fn run(input: &BenchmarkInput, timing: &TimingConfig) -> Result<BenchmarkRun> {
    let module = entry_module(input)?;

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx
        .try_execution_context(0)
        .ok_or_else(|| anyhow!("failed to acquire MonoMove execution guard"))?;

    let mut module_provider = InMemoryModuleProvider::new();
    for (module_id, blob) in input.read_set.modules() {
        module_provider.add_module_bytes(
            module_id.address,
            module_id.name().to_owned(),
            Bytes::from(blob),
        );
    }
    let natives = build_natives(&guard);
    let loader = Loader::new_with_policy(
        &guard,
        &module_provider,
        LoadingPolicy::Lazy(LoweringPolicy::Lazy),
        &natives,
    );

    let total_bytes: usize = input.read_set.data.values().map(|v| v.bytes().len()).sum();
    let arena_size = total_bytes
        .saturating_mul(ARENA_BYTES_PER_RESOURCE_BYTE)
        .max(MIN_ARENA_BYTES);
    let resource_provider = read_set_resource_provider(&guard, &input.read_set, arena_size)?;

    let (transaction_index, reserved_byte) = match input.user_context.transaction_index_kind() {
        TransactionIndexKind::BlockExecution { transaction_index } => (transaction_index, 0),
        TransactionIndexKind::ValidationOrSimulation { transaction_index } => {
            (transaction_index, 1)
        },
        TransactionIndexKind::NotAvailable => (0, 0),
    };
    let mut extensions = NativeExtensions::new();
    extensions.add(TransactionContextExtension::new(
        input.session_id.txn_hash().to_vec(),
        input.session_id.session_counter(),
        transaction_index,
        reserved_byte,
    ));
    extensions.add(ObjectContextExtension::new());
    let usage = input.read_set.get_usage()?;
    extensions.add(StorageUsageAtEpochBoundary::new(
        usage.items() as u64,
        usage.bytes() as u64,
    ));
    extensions.add(EventStore::new());

    // Intern the transaction's type arguments.
    let interned_ty_args = input
        .entry
        .ty_args()
        .iter()
        .map(|tag| intern_type_tag(&guard, tag))
        .collect::<Result<Vec<_>>>()
        .context("failed to intern type arguments")?;
    let ty_arg_list = guard.type_list_of(&interned_ty_args);

    // Load the entry function; this publishes the layouts of the types it touches.
    let module_id = guard
        .intern_address_name(&input.entry.module().address, input.entry.module().name())
        .into_global_arena_ptr();
    let function = guard
        .intern_identifier(input.entry.function())
        .into_global_arena_ptr();
    let mut read_set = ModuleReadSet::new();
    let mut gas_meter = GasMeter::new(GAS_BUDGET);
    let func = match loader.load_function(
        &mut read_set,
        &mut gas_meter,
        module_id,
        function,
        ty_arg_list,
    ) {
        // SAFETY: the pointer lives in a LoadedModule arena kept alive by `guard`.
        Ok(ptr) => unsafe { ptr.as_ref_unchecked() },
        Err(err) => bail!("failed to load entry function on V2: {}", err),
    };

    let mut interp = InterpreterContext::new(
        loader,
        read_set,
        gas_meter,
        &resource_provider,
        &natives,
        func,
    )
    .with_extensions(extensions);

    // Classify each parameter as a signer or a value.
    let params = classify_params(&module, input.entry.function(), &guard, ty_arg_list)?;

    // Sender bytes backing any `&signer` parameter; must outlive every run.
    let signer_bytes = input.sender.into_bytes();

    // Trial run: determine the outcome.
    // TODO(cleanup): need to reset events / extensions?
    interp.reset(func, GAS_BUDGET);
    place_args(
        &mut interp,
        func,
        &params,
        &signer_bytes,
        input.entry.args(),
    )?;
    let outcome = match interp.run() {
        Ok(RuntimeStatus::Success) => {
            // Capture events while the trial run's heap is still live (before the timed runs reset
            // it). SAFETY: the heap objects backing each event value are still live here.
            let events = unsafe { finalize_events_v2(interp.extensions(), &guard) };
            ExecOutcome::Success { events }
        },
        Ok(RuntimeStatus::Aborted { code, message }) => ExecOutcome::Aborted { code, message },
        Err(err) => classify_error(err),
    };

    // Timing: per-run reset is outside the timer; only argument placement + execution are timed.
    let samples = collect_samples(timing, || {
        // TODO(cleanup): need to reset events / extensions?
        interp.reset(func, GAS_BUDGET);
        let start = Instant::now();
        let _ = place_args(
            &mut interp,
            func,
            &params,
            &signer_bytes,
            input.entry.args(),
        );
        let _ = interp.run();
        start.elapsed()
    });

    Ok(BenchmarkRun { outcome, samples })
}

/// The entry function's defining module, deserialized from the read-set.
fn entry_module(input: &BenchmarkInput) -> Result<CompiledModule> {
    let target = input.entry.module();
    for (module_id, bytes) in input.read_set.modules() {
        if &module_id == target {
            return CompiledModule::deserialize(&bytes)
                .map_err(|e| anyhow!("failed to deserialize entry module: {:?}", e));
        }
    }
    bail!("entry module {} not present in the read-set", target)
}

fn classify_params(
    module: &CompiledModule,
    function_name: &IdentStr,
    guard: &ExecutionGuard,
    ty_args: InternedTypeList,
) -> Result<Vec<ParamKind>> {
    for def in module.function_defs() {
        let handle = module.function_handle_at(def.function);
        if module.identifier_at(handle.name) == function_name {
            let signature = module.signature_at(handle.parameters);
            return signature
                .0
                .iter()
                .map(|token| classify_token(guard, module, ty_args, token))
                .collect();
        }
    }
    bail!(
        "entry function {} not found in module {}",
        function_name,
        module.self_id()
    )
}

fn classify_token(
    guard: &ExecutionGuard,
    module: &CompiledModule,
    ty_args: InternedTypeList,
    token: &SignatureToken,
) -> Result<ParamKind> {
    use SignatureToken as S;
    Ok(match token {
        S::Signer => ParamKind::Signer { by_ref: false },
        S::Reference(inner) | S::MutableReference(inner) if matches!(**inner, S::Signer) => {
            ParamKind::Signer { by_ref: true }
        },
        other => ParamKind::Value {
            ty: guard.subst_type(intern_sig_token(other, module, guard)?, ty_args)?,
        },
    })
}

fn place_args(
    interp: &mut InterpreterContext<'_>,
    func: &Function,
    params: &[ParamKind],
    signer_bytes: &[u8],
    entry_args: &[Vec<u8>],
) -> Result<()> {
    if func.param_slots.len() != params.len() {
        bail!(
            "lowered function has {} parameter slots but the signature has {} parameters",
            func.param_slots.len(),
            params.len()
        );
    }
    let mut args = entry_args.iter();
    for (slot, kind) in func.param_slots.iter().zip(params) {
        let offset = slot.offset.0;
        match kind {
            ParamKind::Signer { by_ref: false } => interp.set_root_arg(offset, signer_bytes),
            ParamKind::Signer { by_ref: true } => {
                // A reference is a 16-byte fat pointer (base, byte_offset) pointing at the signer
                // buffer. The base is outside the VM heap, so the GC leaves it alone.
                let mut fat = [0u8; 16];
                fat[..8].copy_from_slice(&(signer_bytes.as_ptr() as u64).to_le_bytes());
                interp.set_root_arg(offset, &fat);
            },
            ParamKind::Value { ty } => {
                let arg = args
                    .next()
                    .ok_or_else(|| anyhow!("not enough arguments for the entry function"))?;
                // SAFETY: `offset`/`ty` come from this function's own signature, so the slot is
                // valid for the type's in-memory size.
                unsafe { interp.deserialize_root_arg(offset, *ty, arg) }.map_err(|e| {
                    anyhow!("failed to place argument at frame offset {}: {}", offset, e)
                })?;
            },
        }
    }
    Ok(())
}

/// Maps a MonoMove runtime error to an [`ExecOutcome::Failure`] with a [`FailureKind`].
fn classify_error(err: RuntimeError) -> ExecOutcome {
    use RuntimeError as E;
    let kind = match &err {
        E::GasExhausted(_) => FailureKind::OutOfGas,
        E::ArithmeticOverflow { .. }
        | E::ArithmeticUnderflow { .. }
        | E::DivisionByZero { .. }
        | E::ShiftAmountOutOfRange { .. }
        | E::ArithmeticUnderOverflow { .. }
        | E::DivisionByZeroOrOverflow { .. }
        | E::NegateMinOverflow { .. }
        | E::CastOutOfRange { .. } => FailureKind::Arithmetic,
        E::PopFromEmptyVector
        | E::VectorIndexOutOfBounds { .. }
        | E::VecUnpackLengthMismatch { .. } => FailureKind::VectorError,
        E::ResourceDoesNotExist { .. } => FailureKind::ResourceDoesNotExist,
        E::ResourceAlreadyExists { .. } => FailureKind::ResourceAlreadyExists,
        E::EnumVariantMismatch { .. } => FailureKind::TypeOrReferenceSafety,
        E::StackOverflow
        | E::OutOfHeapMemory { .. }
        | E::AllocationTooLarge { .. }
        | E::VecAllocSizeOverflow => FailureKind::RuntimeLimitExceeded,
        E::InvalidAbortMessage
        | E::AbortMessageTooLong { .. }
        | E::BCSEof
        | E::BCSInvalidUleb
        | E::BCSSequenceTooLong { .. }
        | E::BCSRemainingInput { .. }
        | E::BCSInvalidBool { .. } => FailureKind::Other,
        E::InvariantViolation(_) | E::ResourceProvider(_) => FailureKind::InvariantViolation,
        E::Loader(loader_err) => classify_loader_error(loader_err),
    };
    ExecOutcome::Failure {
        kind,
        detail: format!("{}", err),
    }
}

/// Maps a loader error to a [`FailureKind`].
fn classify_loader_error(err: &LoaderError) -> FailureKind {
    match err {
        LoaderError::GasExhausted(_) => FailureKind::OutOfGas,
        LoaderError::ModuleNotFound { .. }
        | LoaderError::FunctionNotFound { .. }
        | LoaderError::FunctionIrMissing => FailureKind::Linker,
        LoaderError::LoweringSkipped { .. }
        | LoaderError::Deserialization(_)
        | LoaderError::Verification(_)
        | LoaderError::ModuleProvider(_)
        | LoaderError::GlobalContext(_)
        | LoaderError::Specializer(_) => FailureKind::Other,
        LoaderError::InvariantViolation(_) => FailureKind::InvariantViolation,
    }
}

/// Interns a runtime [`TypeTag`] (e.g. a transaction's type argument, or a resource's struct tag)
/// into a MonoMove [`InternedType`].
/// TODO(cleanup): Move to interner.rs.
pub(crate) fn intern_type_tag(guard: &ExecutionGuard, tag: &TypeTag) -> Result<InternedType> {
    Ok(match tag {
        TypeTag::Bool => BOOL_TY,
        TypeTag::U8 => U8_TY,
        TypeTag::U16 => U16_TY,
        TypeTag::U32 => U32_TY,
        TypeTag::U64 => U64_TY,
        TypeTag::U128 => U128_TY,
        TypeTag::U256 => U256_TY,
        TypeTag::I8 => I8_TY,
        TypeTag::I16 => I16_TY,
        TypeTag::I32 => I32_TY,
        TypeTag::I64 => I64_TY,
        TypeTag::I128 => I128_TY,
        TypeTag::I256 => I256_TY,
        TypeTag::Address => ADDRESS_TY,
        TypeTag::Signer => SIGNER_TY,
        TypeTag::Vector(elem) => guard.vector_of(intern_type_tag(guard, elem)?),
        TypeTag::Struct(struct_tag) => intern_struct_tag(guard, struct_tag)?,
        TypeTag::Function(_) => bail!("function type tags are not supported"),
    })
}

/// Interns a struct tag into its nominal type.
pub(crate) fn intern_struct_tag(
    guard: &ExecutionGuard,
    struct_tag: &StructTag,
) -> Result<InternedType> {
    let module_id = guard.module_id_of(&struct_tag.address, struct_tag.module.as_ident_str());
    let name = guard.identifier_of(struct_tag.name.as_ident_str());
    let args = struct_tag
        .type_args
        .iter()
        .map(|arg| intern_type_tag(guard, arg))
        .collect::<Result<Vec<_>>>()?;
    let ty_args = guard.type_list_of(&args);
    Ok(guard.nominal_of(module_id, name, ty_args))
}

/// Builds an [`InMemoryResourceProvider`] backed by the captured read-set.
fn read_set_resource_provider<'guard, 'ctx>(
    guard: &'guard ExecutionGuard<'ctx>,
    read_set: &ReadSet,
    heap_size: usize,
) -> Result<InMemoryResourceProvider<'guard, 'ctx>> {
    let mut provider = InMemoryResourceProvider::new(guard, heap_size);
    for (state_key, value) in &read_set.data {
        match state_key.inner() {
            StateKeyInner::AccessPath(ap) => match ap.get_path() {
                // Modules are ignored.
                Path::Code(_) => {},
                Path::Resource(struct_tag) => {
                    let ty = intern_struct_tag(guard, &struct_tag)?;
                    provider.add_resource(ap.address, ty, value.bytes().to_vec());
                },
                // A resource group: add each resource in the group individually.
                Path::ResourceGroup(_) => {
                    let members: BTreeMap<StructTag, Vec<u8>> = bcs::from_bytes(value.bytes())?;
                    for (struct_tag, blob) in members {
                        let ty = intern_struct_tag(guard, &struct_tag)?;
                        provider.add_resource(ap.address, ty, blob);
                    }
                },
            },
            StateKeyInner::TableItem { handle, key } => {
                provider.add_table_item(handle.0, key.clone(), value.bytes().to_vec());
            },
            // Neither resources nor table items.
            StateKeyInner::Raw(_) | StateKeyInner::TradingNative(_) => {},
        }
    }
    Ok(provider)
}
