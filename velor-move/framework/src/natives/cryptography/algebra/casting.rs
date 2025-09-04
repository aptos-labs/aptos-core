// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        abort_invariant_violated, AlgebraContext, Structure, BLS12381_R_SCALAR, BN254_R_SCALAR,
        MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    safe_borrow_element, structure_from_ty_arg,
};
use velor_gas_schedule::gas_params::natives::velor_framework::*;
use velor_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use velor_types::on_chain_config::FeatureFlag;
use ark_ff::Field;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use num_traits::One;
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

fn feature_flag_of_casting(
    super_opt: Option<Structure>,
    sub_opt: Option<Structure>,
) -> Option<FeatureFlag> {
    match (super_opt, sub_opt) {
        (Some(Structure::BLS12381Fq12), Some(Structure::BLS12381Gt)) => {
            Some(FeatureFlag::BLS12_381_STRUCTURES)
        },
        (Some(Structure::BN254Fq12), Some(Structure::BN254Gt)) => {
            Some(FeatureFlag::BN254_STRUCTURES)
        },
        _ => None,
    }
}

macro_rules! abort_unless_casting_enabled {
    ($context:ident, $super_opt:expr, $sub_opt:expr) => {
        let flag_opt = feature_flag_of_casting($super_opt, $sub_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

pub fn downcast_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let super_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let sub_opt = structure_from_ty_arg!(context, &ty_args[1]);
    abort_unless_casting_enabled!(context, super_opt, sub_opt);
    match (super_opt, sub_opt) {
        (Some(Structure::BLS12381Fq12), Some(Structure::BLS12381Gt)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(context, handle, ark_bls12_381::Fq12, element_ptr, element);
            context.charge(ALGEBRA_ARK_BLS12_381_FQ12_POW_U256)?;
            if element.pow(BLS12381_R_SCALAR.0) == ark_bls12_381::Fq12::one() {
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            } else {
                Ok(smallvec![Value::bool(false), Value::u64(handle as u64)])
            }
        },
        (Some(Structure::BN254Fq12), Some(Structure::BN254Gt)) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(context, handle, ark_bn254::Fq12, element_ptr, element);
            context.charge(ALGEBRA_ARK_BN254_FQ12_POW_U256)?;
            if element.pow(BN254_R_SCALAR.0) == ark_bn254::Fq12::one() {
                Ok(smallvec![Value::bool(true), Value::u64(handle as u64)])
            } else {
                Ok(smallvec![Value::bool(false), Value::u64(handle as u64)])
            }
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

pub fn upcast_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(2, ty_args.len());
    let sub_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let super_opt = structure_from_ty_arg!(context, &ty_args[1]);
    abort_unless_casting_enabled!(context, super_opt, sub_opt);
    match (sub_opt, super_opt) {
        (Some(Structure::BLS12381Gt), Some(Structure::BLS12381Fq12)) => {
            let handle = safely_pop_arg!(args, u64);
            Ok(smallvec![Value::u64(handle)])
        },
        (Some(Structure::BN254Gt), Some(Structure::BN254Fq12)) => {
            let handle = safely_pop_arg!(args, u64);
            Ok(smallvec![Value::u64(handle)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
