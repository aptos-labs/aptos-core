// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Casts between `Gt` and its enclosing field `Fq12`.
//!
//! Both directions hand the caller's own handle back. The store is heterogeneous
//! and elements are immutable, so the same slot can be read at either type; a
//! cast allocates nothing.

use super::{
    not_implemented, structure_of, AlgebraStore, Structure, BLS12381_R_SCALAR, BN254_R_SCALAR,
    E_CASTING_BLS12381_R_SCALAR_LOADING_FAILED,
};
use ark_ff::Field;
use mono_move_core::{
    native::{NativeContext, NativeStatus},
    VMResult,
};
use num_traits::One;

/// `0x1::crypto_algebra::downcast_internal<Super, Sub>(handle: u64): (bool, u64)`
///
/// An `Fq12` element is in `Gt` exactly when it is an `r`-th root of unity.
/// Failing that test is `(false, handle)`, not an abort.
///
/// TODO(metering): charge gas.
pub fn native_downcast<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let super_structure = structure_of(ctx.ty_arg(0)?);
    let sub_structure = structure_of(ctx.ty_arg(1)?);
    // SAFETY: arg 0 is a `u64` handle.
    let handle = unsafe { ctx.arg::<u64>(0)? };

    let in_subgroup = match (super_structure, sub_structure) {
        (Some(Structure::BLS12381Fq12), Some(Structure::BLS12381Gt)) => {
            let element = ctx
                .get_extension::<AlgebraStore>()?
                .get::<ark_bls12_381::Fq12>(handle)?;
            let Some(r_scalar) = BLS12381_R_SCALAR.as_ref() else {
                return Ok(NativeStatus::Abort {
                    code: E_CASTING_BLS12381_R_SCALAR_LOADING_FAILED,
                    message: Some("BLS12381 R scalar loading failed".to_string()),
                });
            };
            element.pow(r_scalar.0) == ark_bls12_381::Fq12::one()
        },
        (Some(Structure::BN254Fq12), Some(Structure::BN254Gt)) => {
            let element = ctx
                .get_extension::<AlgebraStore>()?
                .get::<ark_bn254::Fq12>(handle)?;
            element.pow(BN254_R_SCALAR.0) == ark_bn254::Fq12::one()
        },
        _ => return Ok(not_implemented()),
    };

    // SAFETY: return slots 0 and 1 are `bool` and `u64`.
    unsafe {
        ctx.set_return(0, in_subgroup)?;
        ctx.set_return(1, handle)?;
    }
    Ok(NativeStatus::Success)
}

/// `0x1::crypto_algebra::upcast_internal<Sub, Super>(handle: u64): u64`
///
/// TODO(metering): charge gas.
pub fn native_upcast<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let sub_structure = structure_of(ctx.ty_arg(0)?);
    let super_structure = structure_of(ctx.ty_arg(1)?);
    // SAFETY: arg 0 is a `u64` handle.
    let handle = unsafe { ctx.arg::<u64>(0)? };

    match (sub_structure, super_structure) {
        (Some(Structure::BLS12381Gt), Some(Structure::BLS12381Fq12))
        | (Some(Structure::BN254Gt), Some(Structure::BN254Fq12)) => {},
        _ => return Ok(not_implemented()),
    }

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}
