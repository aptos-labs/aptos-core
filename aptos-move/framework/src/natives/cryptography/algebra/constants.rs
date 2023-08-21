// Copyright © Aptos Foundation

use crate::{
    abort_unless_arithmetics_enabled_for_structure, abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        feature_flag_from_structure, AlgebraContext, Structure, BLS12381_GT_GENERATOR,
        BLS12381_Q12_LENDIAN, BLS12381_R_LENDIAN, E_TOO_MUCH_MEMORY_USED, MEMORY_LIMIT_IN_BYTES,
        MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    store_element, structure_from_ty_arg,
};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{SafeNativeContext, SafeNativeError, SafeNativeResult};
use ark_ec::Group;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use num_traits::{One, Zero};
use once_cell::sync::Lazy;
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

macro_rules! ark_constant_op_internal {
    ($context:expr, $ark_typ:ty, $ark_func:ident, $gas:expr) => {{
        $context.charge($gas)?;
        let new_element = <$ark_typ>::$ark_func();
        let new_handle = store_element!($context, new_element)?;
        Ok(smallvec![Value::u64(new_handle as u64)])
    }};
}

pub fn zero_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_arithmetics_enabled_for_structure!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_constant_op_internal!(
            context,
            ark_bls12_381::Fr,
            zero,
            ALGEBRA_ARK_BLS12_381_FR_ZERO
        ),
        Some(Structure::BLS12381Fq12) => ark_constant_op_internal!(
            context,
            ark_bls12_381::Fq12,
            zero,
            ALGEBRA_ARK_BLS12_381_FQ12_ZERO
        ),
        Some(Structure::BLS12381G1) => ark_constant_op_internal!(
            context,
            ark_bls12_381::G1Projective,
            zero,
            ALGEBRA_ARK_BLS12_381_G1_PROJ_INFINITY
        ),
        Some(Structure::BLS12381G2) => ark_constant_op_internal!(
            context,
            ark_bls12_381::G2Projective,
            zero,
            ALGEBRA_ARK_BLS12_381_G2_PROJ_INFINITY
        ),
        Some(Structure::BLS12381Gt) => ark_constant_op_internal!(
            context,
            ark_bls12_381::Fq12,
            one,
            ALGEBRA_ARK_BLS12_381_FQ12_ONE
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

pub fn one_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_arithmetics_enabled_for_structure!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_constant_op_internal!(
            context,
            ark_bls12_381::Fr,
            one,
            ALGEBRA_ARK_BLS12_381_FR_ONE
        ),
        Some(Structure::BLS12381Fq12) => ark_constant_op_internal!(
            context,
            ark_bls12_381::Fq12,
            one,
            ALGEBRA_ARK_BLS12_381_FQ12_ONE
        ),
        Some(Structure::BLS12381G1) => ark_constant_op_internal!(
            context,
            ark_bls12_381::G1Projective,
            generator,
            ALGEBRA_ARK_BLS12_381_G1_PROJ_GENERATOR
        ),
        Some(Structure::BLS12381G2) => ark_constant_op_internal!(
            context,
            ark_bls12_381::G2Projective,
            generator,
            ALGEBRA_ARK_BLS12_381_G2_PROJ_GENERATOR
        ),
        Some(Structure::BLS12381Gt) => {
            context.charge(ALGEBRA_ARK_BLS12_381_FQ12_CLONE)?;
            let element = *Lazy::force(&BLS12381_GT_GENERATOR);
            let handle = store_element!(context, element)?;
            Ok(smallvec![Value::u64(handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

pub fn order_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_arithmetics_enabled_for_structure!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr)
        | Some(Structure::BLS12381G1)
        | Some(Structure::BLS12381G2)
        | Some(Structure::BLS12381Gt) => {
            Ok(smallvec![Value::vector_u8(BLS12381_R_LENDIAN.clone())])
        },
        Some(Structure::BLS12381Fq12) => {
            Ok(smallvec![Value::vector_u8(BLS12381_Q12_LENDIAN.clone())])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
