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
use velor_native_interface::{SafeNativeContext, SafeNativeError, SafeNativeResult};
use ark_ff::Field;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

pub fn sqr_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_arithmetics_enabled_for_structure!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_unary_op_internal!(
            context,
            args,
            ark_bls12_381::Fr,
            square,
            ALGEBRA_ARK_BLS12_381_FR_SQUARE
        ),
        Some(Structure::BLS12381Fq12) => ark_unary_op_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            square,
            ALGEBRA_ARK_BLS12_381_FQ12_SQUARE
        ),
        Some(Structure::BN254Fr) => {
            ark_unary_op_internal!(
                context,
                args,
                ark_bn254::Fr,
                square,
                ALGEBRA_ARK_BN254_FR_SQUARE
            )
        },
        Some(Structure::BN254Fq) => {
            ark_unary_op_internal!(
                context,
                args,
                ark_bn254::Fq,
                square,
                ALGEBRA_ARK_BN254_FQ_SQUARE
            )
        },
        Some(Structure::BN254Fq12) => {
            ark_unary_op_internal!(
                context,
                args,
                ark_bn254::Fq12,
                square,
                ALGEBRA_ARK_BN254_FQ12_SQUARE
            )
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
