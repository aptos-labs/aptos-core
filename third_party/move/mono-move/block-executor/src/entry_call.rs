// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Entry-function argument plumbing: type-argument interning, parameter
//! classification, and placing arguments into the root frame. Mirrors the
//! replay-benchmark harness.
// TODO(cleanup): share this with replay-benchmark's `v2.rs`.

use crate::module_provider::StateViewModuleProvider;
use anyhow::{anyhow, bail, Result};
use aptos_types::{
    state_store::{state_key::StateKey, TStateView},
    transaction::EntryFunction,
};
use mono_move_core::{
    intern_sig_token,
    types::{
        InternedType, InternedTypeList, ADDRESS_TY, BOOL_TY, I128_TY, I16_TY, I256_TY, I32_TY,
        I64_TY, I8_TY, SIGNER_TY, U128_TY, U16_TY, U256_TY, U32_TY, U64_TY, U8_TY,
    },
    Function, Interner, ModuleProvider,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{InterpreterContext, TransactionContext};
use move_binary_format::{access::ModuleAccess, file_format::SignatureToken, CompiledModule};
use move_core_types::{
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag},
};

type Interp<'i, 'guard, 'ctx> = InterpreterContext<'i, TransactionContext<'guard, 'ctx>>;

/// How an entry-function parameter is filled into the root frame.
pub(crate) enum ParamKind {
    /// A `signer`/`&signer` parameter, filled with the sender.
    Signer { by_ref: bool },
    /// Any other parameter, deserialized from BCS into the frame.
    Value { ty: InternedType },
}

/// Classifies the entry function's parameters, deserializing its defining
/// module from the base state.
pub(crate) fn classify_entry_params<S: TStateView<Key = StateKey>>(
    module_provider: &StateViewModuleProvider<'_, S>,
    entry: &EntryFunction,
    guard: &ExecutionGuard,
    ty_args: InternedTypeList,
) -> Result<Vec<ParamKind>> {
    let target = entry.module();
    let bytes = module_provider
        .get_module_bytes(target.address(), target.name().as_str())?
        .ok_or_else(|| anyhow!("entry module {} not found in state", target))?;
    let module = module_provider.deserialize_module(&bytes)?;
    classify_params(&module, entry.function(), guard, ty_args)
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

/// Places the entry function's arguments into the root frame: signers from
/// the sender bytes, values deserialized from their BCS blobs.
pub(crate) fn place_args(
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
                // A reference is a 16-byte fat pointer (base, byte_offset)
                // pointing at the signer buffer. The base is outside the VM
                // heap, so the GC leaves it alone.
                let mut fat = [0u8; 16];
                fat[..8].copy_from_slice(&(signer_bytes.as_ptr() as u64).to_le_bytes());
                interp.set_root_arg(offset, &fat);
            },
            ParamKind::Value { ty } => {
                let arg = args
                    .next()
                    .ok_or_else(|| anyhow!("not enough arguments for the entry function"))?;
                // SAFETY: `offset`/`ty` come from this function's own
                // signature, so the slot is valid for the type's in-memory
                // size.
                unsafe { interp.deserialize_root_arg(offset, *ty, arg) }.map_err(|e| {
                    anyhow!("failed to place argument at frame offset {}: {}", offset, e)
                })?;
            },
        }
    }
    Ok(())
}

/// Interns a runtime [`TypeTag`] (e.g. a transaction's type argument) into a
/// MonoMove [`InternedType`].
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
fn intern_struct_tag(guard: &ExecutionGuard, struct_tag: &StructTag) -> Result<InternedType> {
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
