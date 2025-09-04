// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abort_unless_arithmetics_enabled_for_structure, abort_unless_feature_flag_enabled,
    ark_unary_op_internal,
    natives::cryptography::algebra::{
        abort_invariant_violated, feature_flag_from_structure, AlgebraContext, Structure,
        E_TOO_MUCH_MEMORY_USED, MEMORY_LIMIT_IN_BYTES, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    safe_borrow_element, store_element, structure_from_ty_arg,
};
use velor_gas_schedule::gas_params::natives::velor_framework::*;
use velor_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use ark_ff::Field;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, ops::Neg, rc::Rc};

pub fn neg_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_arithmetics_enabled_for_structure!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_unary_op_internal!(
            context,
            args,
            ark_bls12_381::Fr,
            neg,
            ALGEBRA_ARK_BLS12_381_FR_NEG
        ),
        Some(Structure::BLS12381Fq12) => ark_unary_op_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            neg,
            ALGEBRA_ARK_BLS12_381_FQ12_NEG
        ),
        Some(Structure::BLS12381G1) => ark_unary_op_internal!(
            context,
            args,
            ark_bls12_381::G1Projective,
            neg,
            ALGEBRA_ARK_BLS12_381_G1_PROJ_NEG
        ),
        Some(Structure::BLS12381G2) => ark_unary_op_internal!(
            context,
            args,
            ark_bls12_381::G2Projective,
            neg,
            ALGEBRA_ARK_BLS12_381_G2_PROJ_NEG
        ),
        Some(Structure::BLS12381Gt) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(context, handle, ark_bls12_381::Fq12, element_ptr, element);
            context.charge(ALGEBRA_ARK_BLS12_381_FQ12_INV)?;
            let new_element = element.inverse().ok_or_else(abort_invariant_violated)?;
            let new_handle = store_element!(context, new_element)?;
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        Some(Structure::BN254Fr) => {
            ark_unary_op_internal!(context, args, ark_bn254::Fr, neg, ALGEBRA_ARK_BN254_FR_NEG)
        },
        Some(Structure::BN254Fq) => {
            ark_unary_op_internal!(context, args, ark_bn254::Fq, neg, ALGEBRA_ARK_BN254_FQ_NEG)
        },
        Some(Structure::BN254Fq12) => ark_unary_op_internal!(
            context,
            args,
            ark_bn254::Fq12,
            neg,
            ALGEBRA_ARK_BN254_FQ12_NEG
        ),
        Some(Structure::BN254G1) => ark_unary_op_internal!(
            context,
            args,
            ark_bn254::G1Projective,
            neg,
            ALGEBRA_ARK_BN254_G1_PROJ_NEG
        ),
        Some(Structure::BN254G2) => ark_unary_op_internal!(
            context,
            args,
            ark_bn254::G2Projective,
            neg,
            ALGEBRA_ARK_BN254_G2_PROJ_NEG
        ),
        Some(Structure::BN254Gt) => {
            let handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(context, handle, ark_bn254::Fq12, element_ptr, element);
            context.charge(ALGEBRA_ARK_BN254_FQ12_INV)?;
            let new_element = element.inverse().ok_or_else(abort_invariant_violated)?;
            let new_handle = store_element!(context, new_element)?;
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
