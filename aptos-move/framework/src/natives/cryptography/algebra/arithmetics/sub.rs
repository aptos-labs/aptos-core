// Copyright © Aptos Foundation

use crate::{
    abort_unless_arithmetics_enabled_for_structure, abort_unless_feature_flag_enabled,
    ark_binary_op_internal,
    natives::{
        cryptography::algebra::{
            abort_invariant_violated, feature_flag_from_structure, gas::GasParameters,
            AlgebraContext, Structure, E_TOO_MUCH_MEMORY_USED, MEMORY_LIMIT_IN_BYTES,
            MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        },
        helpers::{SafeNativeContext, SafeNativeError, SafeNativeResult},
    },
    safe_borrow_element, safely_pop_arg, store_element, structure_from_ty_arg,
};
use move_core_types::gas_algebra::NumArgs;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{
    collections::VecDeque,
    ops::{Div, Sub},
    rc::Rc,
};

pub fn sub_internal(
    gas_params: &GasParameters,
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_arithmetics_enabled_for_structure!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_binary_op_internal!(
            context,
            args,
            ark_bls12_381::Fr,
            sub,
            gas_params.ark_bls12_381_fr_sub * NumArgs::one()
        ),
        Some(Structure::BLS12381Fq12) => ark_binary_op_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            sub,
            gas_params.ark_bls12_381_fq12_sub * NumArgs::one()
        ),
        Some(Structure::BLS12381G1) => ark_binary_op_internal!(
            context,
            args,
            ark_bls12_381::G1Projective,
            sub,
            gas_params.ark_bls12_381_g1_proj_sub * NumArgs::one()
        ),
        Some(Structure::BLS12381G2) => ark_binary_op_internal!(
            context,
            args,
            ark_bls12_381::G2Projective,
            sub,
            gas_params.ark_bls12_381_g2_proj_sub * NumArgs::one()
        ),
        Some(Structure::BLS12381Gt) => ark_binary_op_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            div,
            gas_params.ark_bls12_381_fq12_div * NumArgs::one()
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
