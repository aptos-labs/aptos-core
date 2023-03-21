// Copyright © Aptos Foundation

use crate::{abort_unless_feature_flag_enabled, abort_unless_arithmetics_enabled_for_structure, ark_binary_op_internal, natives::{
    cryptography::algebra::{
        abort_invariant_violated, gas::GasParameters,
        AlgebraContext, Structure, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    helpers::{SafeNativeContext, SafeNativeError, SafeNativeResult},
}, safe_borrow_element, safely_pop_arg, store_element, structure_from_ty_arg};
use move_core_types::gas_algebra::NumArgs;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, ops::{Sub, Div}, rc::Rc};
use crate::natives::cryptography::algebra::feature_flag_from_structure;

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
        Some(Structure::BLS12381G1Affine) => ark_binary_op_internal!(
            context,
            args,
            ark_bls12_381::G1Projective,
            sub,
            gas_params.ark_bls12_381_g1_proj_sub * NumArgs::one()
        ),
        Some(Structure::BLS12381G2Affine) => ark_binary_op_internal!(
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
