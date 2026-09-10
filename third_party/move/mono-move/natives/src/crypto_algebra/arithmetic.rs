// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Field and group arithmetic natives.
//!
//! `Gt` is `Fq12` written multiplicatively, so its arms reuse the `Fq12` bodies
//! with the operation shifted one level: add is `mul`, sub is `div`, neg is
//! `inverse`, double is `square`.

use super::{algebra_invariant_violation, not_implemented, structure_of, AlgebraStore, Structure};
use ark_ff::{AdditiveGroup, Field};
use mono_move_core::{
    native::{NativeContext, NativeStatus},
    VMResult,
};
use num_traits::Zero;
use std::{any::Any, ops::Div};

/// Stores `op(a, b)` and returns its handle in slot 0.
fn binary_op<C: NativeContext, T: Any + Copy>(
    ctx: &C,
    handle_1: u64,
    handle_2: u64,
    op: impl FnOnce(T, T) -> T,
) -> VMResult<NativeStatus> {
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let element_1 = store.get::<T>(handle_1)?;
    let element_2 = store.get::<T>(handle_2)?;
    let handle = match store.add(op(element_1, element_2)) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// Stores `op(a)` and returns its handle in slot 0.
fn unary_op<C: NativeContext, T: Any + Copy>(
    ctx: &C,
    handle: u64,
    op: impl FnOnce(T) -> T,
) -> VMResult<NativeStatus> {
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let element = store.get::<T>(handle)?;
    let handle = match store.add(op(element)) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// [`unary_op`] where `op` returning [`None`] is an invariant violation.
fn unary_op_or_violation<C: NativeContext, T: Any + Copy>(
    ctx: &C,
    handle: u64,
    op: impl FnOnce(T) -> Option<T>,
) -> VMResult<NativeStatus> {
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let element = store.get::<T>(handle)?;
    let Some(new_element) = op(element) else {
        return algebra_invariant_violation();
    };
    let handle = match store.add(new_element) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::add_internal<S>(handle_1: u64, handle_2: u64): u64`
///
/// TODO(metering): charge gas.
pub fn native_add<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    // SAFETY: args 0 and 1 are `u64` handles.
    let h1 = unsafe { ctx.arg::<u64>(0)? };
    let h2 = unsafe { ctx.arg::<u64>(1)? };

    match structure {
        Some(Structure::BLS12381Fr) => binary_op(ctx, h1, h2, |a: ark_bls12_381::Fr, b| a + b),
        Some(Structure::BLS12381Fq12) => binary_op(ctx, h1, h2, |a: ark_bls12_381::Fq12, b| a + b),
        Some(Structure::BLS12381G1) => {
            binary_op(ctx, h1, h2, |a: ark_bls12_381::G1Projective, b| a + b)
        },
        Some(Structure::BLS12381G2) => {
            binary_op(ctx, h1, h2, |a: ark_bls12_381::G2Projective, b| a + b)
        },
        Some(Structure::BLS12381Gt) => binary_op(ctx, h1, h2, |a: ark_bls12_381::Fq12, b| a * b),
        Some(Structure::BN254Fr) => binary_op(ctx, h1, h2, |a: ark_bn254::Fr, b| a + b),
        Some(Structure::BN254Fq) => binary_op(ctx, h1, h2, |a: ark_bn254::Fq, b| a + b),
        Some(Structure::BN254Fq12) => binary_op(ctx, h1, h2, |a: ark_bn254::Fq12, b| a + b),
        Some(Structure::BN254G1) => binary_op(ctx, h1, h2, |a: ark_bn254::G1Projective, b| a + b),
        Some(Structure::BN254G2) => binary_op(ctx, h1, h2, |a: ark_bn254::G2Projective, b| a + b),
        Some(Structure::BN254Gt) => binary_op(ctx, h1, h2, |a: ark_bn254::Fq12, b| a * b),
        None => Ok(not_implemented()),
    }
}

/// `0x1::crypto_algebra::sub_internal<G>(handle_1: u64, handle_2: u64): u64`
///
/// TODO(metering): charge gas.
pub fn native_sub<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    // SAFETY: args 0 and 1 are `u64` handles.
    let h1 = unsafe { ctx.arg::<u64>(0)? };
    let h2 = unsafe { ctx.arg::<u64>(1)? };

    match structure {
        Some(Structure::BLS12381Fr) => binary_op(ctx, h1, h2, |a: ark_bls12_381::Fr, b| a - b),
        Some(Structure::BLS12381Fq12) => binary_op(ctx, h1, h2, |a: ark_bls12_381::Fq12, b| a - b),
        Some(Structure::BLS12381G1) => {
            binary_op(ctx, h1, h2, |a: ark_bls12_381::G1Projective, b| a - b)
        },
        Some(Structure::BLS12381G2) => {
            binary_op(ctx, h1, h2, |a: ark_bls12_381::G2Projective, b| a - b)
        },
        Some(Structure::BLS12381Gt) => binary_op(ctx, h1, h2, |a: ark_bls12_381::Fq12, b| a / b),
        Some(Structure::BN254Fr) => binary_op(ctx, h1, h2, |a: ark_bn254::Fr, b| a - b),
        Some(Structure::BN254Fq) => binary_op(ctx, h1, h2, |a: ark_bn254::Fq, b| a - b),
        Some(Structure::BN254Fq12) => binary_op(ctx, h1, h2, |a: ark_bn254::Fq12, b| a - b),
        Some(Structure::BN254G1) => binary_op(ctx, h1, h2, |a: ark_bn254::G1Projective, b| a - b),
        Some(Structure::BN254G2) => binary_op(ctx, h1, h2, |a: ark_bn254::G2Projective, b| a - b),
        Some(Structure::BN254Gt) => binary_op(ctx, h1, h2, |a: ark_bn254::Fq12, b| a / b),
        None => Ok(not_implemented()),
    }
}

/// `0x1::crypto_algebra::mul_internal<F>(handle_1: u64, handle_2: u64): u64`
///
/// Fields only — a group's multiplicative operation is `add_internal`.
///
/// TODO(metering): charge gas.
pub fn native_mul<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    // SAFETY: args 0 and 1 are `u64` handles.
    let h1 = unsafe { ctx.arg::<u64>(0)? };
    let h2 = unsafe { ctx.arg::<u64>(1)? };

    match structure {
        Some(Structure::BLS12381Fr) => binary_op(ctx, h1, h2, |a: ark_bls12_381::Fr, b| a * b),
        Some(Structure::BLS12381Fq12) => binary_op(ctx, h1, h2, |a: ark_bls12_381::Fq12, b| a * b),
        Some(Structure::BN254Fr) => binary_op(ctx, h1, h2, |a: ark_bn254::Fr, b| a * b),
        Some(Structure::BN254Fq) => binary_op(ctx, h1, h2, |a: ark_bn254::Fq, b| a * b),
        Some(Structure::BN254Fq12) => binary_op(ctx, h1, h2, |a: ark_bn254::Fq12, b| a * b),
        Some(Structure::BLS12381G1)
        | Some(Structure::BLS12381G2)
        | Some(Structure::BLS12381Gt)
        | Some(Structure::BN254G1)
        | Some(Structure::BN254G2)
        | Some(Structure::BN254Gt)
        | None => Ok(not_implemented()),
    }
}

/// Divides two stored field elements, returning `(true, handle)` or, for a zero
/// divisor, `(false, 0)`.
fn div_op<C: NativeContext, T: Any + Copy + Zero + Div<Output = T>>(
    ctx: &C,
    handle_1: u64,
    handle_2: u64,
) -> VMResult<NativeStatus> {
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let element_1 = store.get::<T>(handle_1)?;
    let element_2 = store.get::<T>(handle_2)?;
    if element_2.is_zero() {
        drop(store);
        // SAFETY: return slots 0 and 1 are `bool` and `u64`.
        unsafe {
            ctx.set_return(0, false)?;
            ctx.set_return(1, 0u64)?;
        }
        return Ok(NativeStatus::Success);
    }
    let handle = match store.add(element_1 / element_2) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slots 0 and 1 are `bool` and `u64`.
    unsafe {
        ctx.set_return(0, true)?;
        ctx.set_return(1, handle)?;
    }
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::div_internal<F>(handle_1: u64, handle_2: u64): (bool, u64)`
///
/// TODO(metering): charge gas.
pub fn native_div<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    // SAFETY: args 0 and 1 are `u64` handles.
    let h1 = unsafe { ctx.arg::<u64>(0)? };
    let h2 = unsafe { ctx.arg::<u64>(1)? };

    match structure {
        Some(Structure::BLS12381Fr) => div_op::<_, ark_bls12_381::Fr>(ctx, h1, h2),
        Some(Structure::BLS12381Fq12) => div_op::<_, ark_bls12_381::Fq12>(ctx, h1, h2),
        Some(Structure::BN254Fr) => div_op::<_, ark_bn254::Fr>(ctx, h1, h2),
        Some(Structure::BN254Fq) => div_op::<_, ark_bn254::Fq>(ctx, h1, h2),
        Some(Structure::BN254Fq12) => div_op::<_, ark_bn254::Fq12>(ctx, h1, h2),
        Some(Structure::BLS12381G1)
        | Some(Structure::BLS12381G2)
        | Some(Structure::BLS12381Gt)
        | Some(Structure::BN254G1)
        | Some(Structure::BN254G2)
        | Some(Structure::BN254Gt)
        | None => Ok(not_implemented()),
    }
}

/// `0x1::crypto_algebra::neg_internal<F>(handle: u64): u64`
///
/// TODO(metering): charge gas.
pub fn native_neg<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    // SAFETY: arg 0 is a `u64` handle.
    let h = unsafe { ctx.arg::<u64>(0)? };

    match structure {
        Some(Structure::BLS12381Fr) => unary_op(ctx, h, |a: ark_bls12_381::Fr| -a),
        Some(Structure::BLS12381Fq12) => unary_op(ctx, h, |a: ark_bls12_381::Fq12| -a),
        Some(Structure::BLS12381G1) => unary_op(ctx, h, |a: ark_bls12_381::G1Projective| -a),
        Some(Structure::BLS12381G2) => unary_op(ctx, h, |a: ark_bls12_381::G2Projective| -a),
        Some(Structure::BLS12381Gt) => {
            unary_op_or_violation(ctx, h, |a: ark_bls12_381::Fq12| a.inverse())
        },
        Some(Structure::BN254Fr) => unary_op(ctx, h, |a: ark_bn254::Fr| -a),
        Some(Structure::BN254Fq) => unary_op(ctx, h, |a: ark_bn254::Fq| -a),
        Some(Structure::BN254Fq12) => unary_op(ctx, h, |a: ark_bn254::Fq12| -a),
        Some(Structure::BN254G1) => unary_op(ctx, h, |a: ark_bn254::G1Projective| -a),
        Some(Structure::BN254G2) => unary_op(ctx, h, |a: ark_bn254::G2Projective| -a),
        Some(Structure::BN254Gt) => unary_op_or_violation(ctx, h, |a: ark_bn254::Fq12| a.inverse()),
        None => Ok(not_implemented()),
    }
}

/// Inverts a stored field element, returning `(true, handle)` or, when the
/// element has no inverse, `(false, 0)`.
fn inv_op<C: NativeContext, T: Any + Copy + Field>(ctx: &C, handle: u64) -> VMResult<NativeStatus> {
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let element = store.get::<T>(handle)?;
    let Some(new_element) = element.inverse() else {
        drop(store);
        // SAFETY: return slots 0 and 1 are `bool` and `u64`.
        unsafe {
            ctx.set_return(0, false)?;
            ctx.set_return(1, 0u64)?;
        }
        return Ok(NativeStatus::Success);
    };
    let handle = match store.add(new_element) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slots 0 and 1 are `bool` and `u64`.
    unsafe {
        ctx.set_return(0, true)?;
        ctx.set_return(1, handle)?;
    }
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::inv_internal<F>(handle: u64): (bool, u64)`
///
/// TODO(metering): charge gas.
pub fn native_inv<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    // SAFETY: arg 0 is a `u64` handle.
    let h = unsafe { ctx.arg::<u64>(0)? };

    match structure {
        Some(Structure::BLS12381Fr) => inv_op::<_, ark_bls12_381::Fr>(ctx, h),
        Some(Structure::BLS12381Fq12) => inv_op::<_, ark_bls12_381::Fq12>(ctx, h),
        Some(Structure::BN254Fr) => inv_op::<_, ark_bn254::Fr>(ctx, h),
        Some(Structure::BN254Fq) => inv_op::<_, ark_bn254::Fq>(ctx, h),
        Some(Structure::BN254Fq12) => inv_op::<_, ark_bn254::Fq12>(ctx, h),
        Some(Structure::BLS12381G1)
        | Some(Structure::BLS12381G2)
        | Some(Structure::BLS12381Gt)
        | Some(Structure::BN254G1)
        | Some(Structure::BN254G2)
        | Some(Structure::BN254Gt)
        | None => Ok(not_implemented()),
    }
}

/// `0x1::crypto_algebra::sqr_internal<G>(handle: u64): u64`
///
/// Fields only, unlike `double_internal`, which covers groups and `Gt`.
///
/// TODO(metering): charge gas.
pub fn native_sqr<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    // SAFETY: arg 0 is a `u64` handle.
    let h = unsafe { ctx.arg::<u64>(0)? };

    match structure {
        Some(Structure::BLS12381Fr) => unary_op(ctx, h, |a: ark_bls12_381::Fr| a.square()),
        Some(Structure::BLS12381Fq12) => unary_op(ctx, h, |a: ark_bls12_381::Fq12| a.square()),
        Some(Structure::BN254Fr) => unary_op(ctx, h, |a: ark_bn254::Fr| a.square()),
        Some(Structure::BN254Fq) => unary_op(ctx, h, |a: ark_bn254::Fq| a.square()),
        Some(Structure::BN254Fq12) => unary_op(ctx, h, |a: ark_bn254::Fq12| a.square()),
        Some(Structure::BLS12381G1)
        | Some(Structure::BLS12381G2)
        | Some(Structure::BLS12381Gt)
        | Some(Structure::BN254G1)
        | Some(Structure::BN254G2)
        | Some(Structure::BN254Gt)
        | None => Ok(not_implemented()),
    }
}

/// `0x1::crypto_algebra::double_internal<G>(element_handle: u64): u64`
///
/// Groups and `Gt` only, unlike `sqr_internal`, which covers fields.
///
/// TODO(metering): charge gas.
pub fn native_double<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    // SAFETY: arg 0 is a `u64` handle.
    let h = unsafe { ctx.arg::<u64>(0)? };

    match structure {
        Some(Structure::BLS12381G1) => {
            unary_op(ctx, h, |a: ark_bls12_381::G1Projective| a.double())
        },
        Some(Structure::BLS12381G2) => {
            unary_op(ctx, h, |a: ark_bls12_381::G2Projective| a.double())
        },
        Some(Structure::BLS12381Gt) => unary_op(ctx, h, |a: ark_bls12_381::Fq12| a.square()),
        Some(Structure::BN254G1) => unary_op(ctx, h, |a: ark_bn254::G1Projective| a.double()),
        Some(Structure::BN254G2) => unary_op(ctx, h, |a: ark_bn254::G2Projective| a.double()),
        Some(Structure::BN254Gt) => unary_op(ctx, h, |a: ark_bn254::Fq12| a.square()),
        Some(Structure::BLS12381Fr)
        | Some(Structure::BLS12381Fq12)
        | Some(Structure::BN254Fr)
        | Some(Structure::BN254Fq)
        | Some(Structure::BN254Fq12)
        | None => Ok(not_implemented()),
    }
}

/// Compares two stored elements and returns the result in slot 0.
fn eq_op<C: NativeContext, T: Any + Copy + PartialEq>(
    ctx: &C,
    handle_1: u64,
    handle_2: u64,
) -> VMResult<NativeStatus> {
    let result = {
        let store = ctx.get_extension::<AlgebraStore>()?;
        store.get::<T>(handle_1)? == store.get::<T>(handle_2)?
    };

    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, result)? };
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::eq_internal<S>(handle_1: u64, handle_2: u64): bool`
///
/// TODO(metering): charge gas.
pub fn native_eq<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    // SAFETY: args 0 and 1 are `u64` handles.
    let h1 = unsafe { ctx.arg::<u64>(0)? };
    let h2 = unsafe { ctx.arg::<u64>(1)? };

    match structure {
        Some(Structure::BLS12381Fr) => eq_op::<_, ark_bls12_381::Fr>(ctx, h1, h2),
        Some(Structure::BLS12381Fq12) => eq_op::<_, ark_bls12_381::Fq12>(ctx, h1, h2),
        Some(Structure::BLS12381G1) => eq_op::<_, ark_bls12_381::G1Projective>(ctx, h1, h2),
        Some(Structure::BLS12381G2) => eq_op::<_, ark_bls12_381::G2Projective>(ctx, h1, h2),
        Some(Structure::BLS12381Gt) => eq_op::<_, ark_bls12_381::Fq12>(ctx, h1, h2),
        Some(Structure::BN254Fr) => eq_op::<_, ark_bn254::Fr>(ctx, h1, h2),
        Some(Structure::BN254Fq) => eq_op::<_, ark_bn254::Fq>(ctx, h1, h2),
        Some(Structure::BN254Fq12) => eq_op::<_, ark_bn254::Fq12>(ctx, h1, h2),
        Some(Structure::BN254G1) => eq_op::<_, ark_bn254::G1Projective>(ctx, h1, h2),
        Some(Structure::BN254G2) => eq_op::<_, ark_bn254::G2Projective>(ctx, h1, h2),
        Some(Structure::BN254Gt) => eq_op::<_, ark_bn254::Fq12>(ctx, h1, h2),
        None => Ok(not_implemented()),
    }
}

/// Stores `T::from(value)` and returns its handle in slot 0.
fn from_u64_op<C: NativeContext, T: Any + From<u64>>(
    ctx: &C,
    value: u64,
) -> VMResult<NativeStatus> {
    let mut store = ctx.get_extension::<AlgebraStore>()?;
    let handle = match store.add(T::from(value)) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::from_u64_internal<S>(value: u64): u64`
///
/// TODO(metering): charge gas.
pub fn native_from_u64<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let structure = structure_of(ctx.ty_arg(0)?);
    // SAFETY: arg 0 is `u64`.
    let value = unsafe { ctx.arg::<u64>(0)? };

    match structure {
        Some(Structure::BLS12381Fr) => from_u64_op::<_, ark_bls12_381::Fr>(ctx, value),
        Some(Structure::BLS12381Fq12) => from_u64_op::<_, ark_bls12_381::Fq12>(ctx, value),
        Some(Structure::BN254Fr) => from_u64_op::<_, ark_bn254::Fr>(ctx, value),
        Some(Structure::BN254Fq) => from_u64_op::<_, ark_bn254::Fq>(ctx, value),
        Some(Structure::BN254Fq12) => from_u64_op::<_, ark_bn254::Fq12>(ctx, value),
        Some(Structure::BLS12381G1)
        | Some(Structure::BLS12381G2)
        | Some(Structure::BLS12381Gt)
        | Some(Structure::BN254G1)
        | Some(Structure::BN254G2)
        | Some(Structure::BN254Gt)
        | None => Ok(not_implemented()),
    }
}
