// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `reflect` module.

use crate::{polymorphic_natives, NativeEntry};
use aptos_types::move_stdlib::{
    reflect::{is_forbidden_to_reflect, INVALID_IDENTIFIER},
    result::{ERR_TAG as RESULT_ERR_TAG, OK_TAG as RESULT_OK_TAG},
};
use mono_move_core::{
    native::{
        native_invariant_violation, FunctionResolutionError, NativeContext, NativeContextFamily,
        NativeStatus, Ref, Vector,
    },
    DescriptorId, VMResult, TRIVIAL_DESCRIPTOR_ID,
};
use move_core_types::{account_address::AccountAddress, identifier::Identifier};

/// `0x1::reflect::native_resolve<FuncType>(addr: address, module_name: &String,
/// func_name: &String): Result<FuncType, ReflectionError>`
///
/// Resolves a public function named at runtime into a function value of type
/// `FuncType`, inferring the target's type arguments from `FuncType`. Every way
/// this can fail is a `ReflectionError` the caller sees, not an abort.
//
// TODO(metering): charge gas.
//
// TODO(completeness): a `native fun` target resolves here but then fails at the
// call with a linking error, exactly as a directly packed closure over a native
// already does. The legacy VM calls it.
pub fn native_resolve<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let expected_ty = ctx.ty_arg(0)?;
    // SAFETY: arg 0 is `address`.
    let address: AccountAddress = unsafe { ctx.arg(0)? };
    let (Some(module_name), Some(func_name)) = (read_name(ctx, 1)?, read_name(ctx, 2)?) else {
        return err_result(ctx, INVALID_IDENTIFIER as u64);
    };

    if is_forbidden_to_reflect(&address, &module_name, &func_name) {
        return err_result(ctx, FunctionResolutionError::FunctionNotAccessible as u64);
    }

    let func = match ctx.resolve_function(address, &module_name, &func_name, expected_ty)? {
        Ok(func) => func,
        Err(err) => return err_result(ctx, err as u64),
    };
    // SAFETY: `func` has type `FuncType`, the `Ok` payload of return 0.
    let result = unsafe { ctx.new_enum(result_descriptor(ctx)?, RESULT_OK_TAG as u64, func)? };
    // SAFETY: return 0 is `Result<FuncType, ReflectionError>`.
    unsafe { ctx.set_return(0, result)? };
    Ok(NativeStatus::Success)
}

/// Reads the `&String` argument at `i` as a Move identifier, or [`None`] if its
/// bytes do not spell one.
fn read_name<C: NativeContext>(ctx: &C, i: usize) -> VMResult<Option<Identifier>> {
    // SAFETY: the argument is `&String`, flattened to `vector<u8>`.
    let s: Ref<Vector<u8>> = unsafe { ctx.arg(i)? };
    let v = s.borrow();
    // SAFETY: the bytes are copied out immediately, so GC cannot relocate them
    // while the slice is held.
    let bytes = unsafe { v.as_bytes() }.to_vec();
    Ok(Identifier::from_utf8(bytes).ok())
}

/// Returns `Result::Err(code)` from the native.
fn err_result<C: NativeContext>(ctx: &C, code: u64) -> VMResult<NativeStatus> {
    let descriptor = result_descriptor(ctx)?;
    // `ReflectionError`'s variants are all fieldless, so the value is
    // pointer-free and built under the trivial descriptor.
    //
    // SAFETY: the built value is a `ReflectionError`, the `Err` payload.
    let err = unsafe { ctx.new_enum(TRIVIAL_DESCRIPTOR_ID, code, ())? };
    // SAFETY: `err` is rooted, so the allocation below may collect.
    let result = unsafe { ctx.new_enum(descriptor, RESULT_ERR_TAG as u64, err)? };
    // SAFETY: return 0 is `Result<FuncType, ReflectionError>`.
    unsafe { ctx.set_return(0, result)? };
    Ok(NativeStatus::Success)
}

/// The object descriptor of the native's `Result<FuncType, ReflectionError>`
/// return type, published when the call site was lowered.
fn result_descriptor<C: NativeContext>(ctx: &C) -> VMResult<DescriptorId> {
    let ty = ctx.return_type(0)?;
    ctx.enum_descriptor(ty).ok_or_else(|| {
        native_invariant_violation(
            "Descriptor ID for result type of `reflect::native_resolve` is not found".to_string(),
        )
    })
}

/// Natives for the `reflect` module.
pub fn make_all_reflect_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![("0x1::reflect::native_resolve", native_resolve)]
}
