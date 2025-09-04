// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abort_unless_arithmetics_enabled_for_structure, abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        abort_invariant_violated, feature_flag_from_structure, AlgebraContext, Structure,
        MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    safe_borrow_element, structure_from_ty_arg,
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

macro_rules! ark_eq_internal {
    ($context:ident, $args:ident, $ark_typ:ty, $gas:expr_2021) => {{
        let handle_2 = safely_pop_arg!($args, u64) as usize;
        let handle_1 = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle_1, $ark_typ, element_1_ptr, element_1);
        safe_borrow_element!($context, handle_2, $ark_typ, element_2_ptr, element_2);
        $context.charge($gas)?;
        let result = element_1 == element_2;
        Ok(smallvec![Value::bool(result)])
    }};
}

pub fn eq_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_arithmetics_enabled_for_structure!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_eq_internal!(
            context,
            args,
            ark_bls12_381::Fr,
            ALGEBRA_ARK_BLS12_381_FR_EQ
        ),
        Some(Structure::BLS12381Fq12) => ark_eq_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            ALGEBRA_ARK_BLS12_381_FQ12_EQ
        ),
        Some(Structure::BLS12381G1) => ark_eq_internal!(
            context,
            args,
            ark_bls12_381::G1Projective,
            ALGEBRA_ARK_BLS12_381_G1_PROJ_EQ
        ),
        Some(Structure::BLS12381G2) => ark_eq_internal!(
            context,
            args,
            ark_bls12_381::G2Projective,
            ALGEBRA_ARK_BLS12_381_G2_PROJ_EQ
        ),
        Some(Structure::BLS12381Gt) => ark_eq_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            ALGEBRA_ARK_BLS12_381_FQ12_EQ
        ),
        Some(Structure::BN254Fr) => {
            ark_eq_internal!(context, args, ark_bn254::Fr, ALGEBRA_ARK_BN254_FR_EQ)
        },
        Some(Structure::BN254Fq) => {
            ark_eq_internal!(context, args, ark_bn254::Fq, ALGEBRA_ARK_BN254_FQ_EQ)
        },
        Some(Structure::BN254Fq12) => {
            ark_eq_internal!(context, args, ark_bn254::Fq12, ALGEBRA_ARK_BN254_FQ12_EQ)
        },
        Some(Structure::BN254G1) => {
            ark_eq_internal!(
                context,
                args,
                ark_bn254::G1Projective,
                ALGEBRA_ARK_BN254_G1_PROJ_EQ
            )
        },
        Some(Structure::BN254G2) => {
            ark_eq_internal!(
                context,
                args,
                ark_bn254::G2Projective,
                ALGEBRA_ARK_BN254_G2_PROJ_EQ
            )
        },
        Some(Structure::BN254Gt) => {
            ark_eq_internal!(context, args, ark_bn254::Fq12, ALGEBRA_ARK_BN254_FQ12_EQ)
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
