// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! MonoMove (V2) harness: runs a transaction's entry function on MonoMove and returns its outcome
//! and timing.

use crate::{
    compare::{ExecOutcome, FailureKind},
    data::BenchmarkInput,
    resource::ReadSetResourceProvider,
    timing::{collect_samples, TimingConfig},
    BenchmarkRun,
};
use anyhow::{anyhow, bail, Context, Result};
use bytes::Bytes;
use mono_move_core::{
    native::NativeExtensions,
    types::{
        InternedType, ADDRESS_TY, BOOL_TY, I128_TY, I16_TY, I256_TY, I32_TY, I64_TY, I8_TY,
        SIGNER_TY, U128_TY, U16_TY, U256_TY, U32_TY, U64_TY, U8_TY,
    },
    Function, GasMeter, Interner, LoaderError,
};
use mono_move_global_context::{ExecutionGuard, GlobalContext};
use mono_move_loader::{Loader, LoadingPolicy, LoweringPolicy};
use mono_move_natives::{
    EventStore, ObjectContextExtension, StorageUsageAtEpochBoundary, TransactionContextExtension,
};
use mono_move_runtime::{
    ExecutionContext, InterpreterContext, RuntimeError, RuntimeStatus, TransactionContext,
};
use mono_move_testsuite::{build_natives, InMemoryModuleProvider};
use move_binary_format::{
    access::ModuleAccess,
    file_format::{SignatureToken, StructHandleIndex},
    CompiledModule,
};
use move_core_types::{identifier::IdentStr, language_storage::TypeTag};
use std::time::Instant;

/// Effectively unbounded gas budget.
const GAS_BUDGET: u64 = u64::MAX;

/// The resource arena is sized as `resource bytes * ARENA_BYTES_PER_RESOURCE_BYTE`, with a floor of
/// `MIN_ARENA_BYTES` (the flat representation can be larger than BCS).
const MIN_ARENA_BYTES: usize = 16 * 1024 * 1024;
const ARENA_BYTES_PER_RESOURCE_BYTE: usize = 8;

type Interp<'i, 'guard, 'ctx> = InterpreterContext<'i, TransactionContext<'guard, 'ctx>>;

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

    let resources = input.read_set.resources()?;
    let total_resource_bytes: usize = resources.iter().map(|r| r.blob.len()).sum();
    let arena_size = total_resource_bytes
        .saturating_mul(ARENA_BYTES_PER_RESOURCE_BYTE)
        .max(MIN_ARENA_BYTES);
    let resource_provider = ReadSetResourceProvider::new(&guard, &resources, arena_size);

    let mut extensions = NativeExtensions::new();
    extensions.add(TransactionContextExtension::new(vec![0u8; 32], 0, 0, 0));
    extensions.add(ObjectContextExtension::new());
    extensions.add(StorageUsageAtEpochBoundary::new(0, 0));
    extensions.add(EventStore::new());

    let mut txn_ctx = TransactionContext::new(
        loader,
        GasMeter::new(GAS_BUDGET),
        &resource_provider,
        &natives,
    )
    .with_extensions(extensions);

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
    let func = match txn_ctx.load_function(module_id, function, ty_arg_list) {
        // SAFETY: the pointer lives in a LoadedModule arena kept alive by `guard`.
        Ok(ptr) => unsafe { ptr.as_ref_unchecked() },
        Err(err) => bail!("failed to load entry function on V2: {}", err),
    };

    // Classify each parameter as a signer or a value.
    let params = classify_params(&module, input.entry.function(), &guard, &interned_ty_args)?;

    // Sender bytes backing any `&signer` parameter; must outlive every run.
    let signer_bytes = input.sender.into_bytes();

    let mut interp = InterpreterContext::new(&mut txn_ctx, func);

    // Trial run: determine the outcome.
    interp.reset(func, GAS_BUDGET);
    place_args(
        &mut interp,
        func,
        &params,
        &signer_bytes,
        input.entry.args(),
    )?;
    let outcome = match interp.run() {
        Ok(RuntimeStatus::Success) => ExecOutcome::Success,
        Ok(RuntimeStatus::Aborted { code, message }) => ExecOutcome::Aborted { code, message },
        Err(err) => classify_error(err),
    };

    // Timing: per-run reset is outside the timer; only argument placement + execution are timed.
    let samples = collect_samples(timing, || {
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
    ty_args: &[InternedType],
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
    ty_args: &[InternedType],
    token: &SignatureToken,
) -> Result<ParamKind> {
    use SignatureToken as S;
    Ok(match token {
        S::Signer => ParamKind::Signer { by_ref: false },
        S::Reference(inner) | S::MutableReference(inner) if matches!(**inner, S::Signer) => {
            ParamKind::Signer { by_ref: true }
        },
        other => ParamKind::Value {
            ty: intern_signature_token(guard, module, ty_args, other)?,
        },
    })
}

fn place_args(
    interp: &mut Interp<'_, '_, '_>,
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
        TypeTag::Struct(struct_tag) => {
            let module_id =
                guard.module_id_of(&struct_tag.address, struct_tag.module.as_ident_str());
            let name = guard.identifier_of(struct_tag.name.as_ident_str());
            let args = struct_tag
                .type_args
                .iter()
                .map(|arg| intern_type_tag(guard, arg))
                .collect::<Result<Vec<_>>>()?;
            let ty_args = guard.type_list_of(&args);
            guard.nominal_of(module_id, name, ty_args)
        },
        TypeTag::Function(_) => bail!("function types are not supported for interning"),
    })
}

/// Interns a [`SignatureToken`] (a parameter/field type read from a module's bytecode) into a
/// MonoMove [`InternedType`], resolving struct handles against `module` and substituting type
/// parameters with the (already-interned) `ty_args`.
fn intern_signature_token(
    guard: &ExecutionGuard,
    module: &CompiledModule,
    ty_args: &[InternedType],
    token: &SignatureToken,
) -> Result<InternedType> {
    use SignatureToken as S;
    Ok(match token {
        S::Bool => BOOL_TY,
        S::U8 => U8_TY,
        S::U16 => U16_TY,
        S::U32 => U32_TY,
        S::U64 => U64_TY,
        S::U128 => U128_TY,
        S::U256 => U256_TY,
        S::I8 => I8_TY,
        S::I16 => I16_TY,
        S::I32 => I32_TY,
        S::I64 => I64_TY,
        S::I128 => I128_TY,
        S::I256 => I256_TY,
        S::Address => ADDRESS_TY,
        S::Signer => SIGNER_TY,
        S::Vector(elem) => guard.vector_of(intern_signature_token(guard, module, ty_args, elem)?),
        S::Reference(inner) => {
            guard.immut_ref_of(intern_signature_token(guard, module, ty_args, inner)?)
        },
        S::MutableReference(inner) => {
            guard.mut_ref_of(intern_signature_token(guard, module, ty_args, inner)?)
        },
        S::TypeParameter(idx) => *ty_args
            .get(*idx as usize)
            .ok_or_else(|| anyhow::anyhow!("type parameter {} out of range", idx))?,
        S::Struct(handle) => intern_struct(guard, module, *handle, &[])?,
        S::StructInstantiation(handle, args) => {
            let args = args
                .iter()
                .map(|arg| intern_signature_token(guard, module, ty_args, arg))
                .collect::<Result<Vec<_>>>()?;
            intern_struct(guard, module, *handle, &args)?
        },
        S::Function(..) => bail!("function types are not supported for interning"),
    })
}

fn intern_struct(
    guard: &ExecutionGuard,
    module: &CompiledModule,
    handle: StructHandleIndex,
    args: &[InternedType],
) -> Result<InternedType> {
    let struct_handle = module.struct_handle_at(handle);
    let module_handle = module.module_handle_at(struct_handle.module);
    let address = module.address_identifier_at(module_handle.address);
    let module_name = module.identifier_at(module_handle.name);
    let struct_name = module.identifier_at(struct_handle.name);

    let module_id = guard.module_id_of(address, module_name);
    let name = guard.identifier_of(struct_name);
    let ty_args = guard.type_list_of(args);
    Ok(guard.nominal_of(module_id, name, ty_args))
}
