// Copyright © Aptos Foundation

use crate::{
    abort_unless_arithmetics_enabled_for_structure, abort_unless_feature_flag_enabled,
    natives::{
        cryptography::algebra::{
            abort_invariant_violated, feature_flag_from_structure, gas::GasParameters,
            AlgebraContext, Structure, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        },
        helpers::{SafeNativeContext, SafeNativeError, SafeNativeResult},
    },
    safe_borrow_element, safely_pop_arg, structure_from_ty_arg,
};
use move_core_types::gas_algebra::NumArgs;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

macro_rules! ark_eq_internal {
    ($context:ident, $args:ident, $ark_typ:ty, $gas:expr) => {{
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
    gas_params: &GasParameters,
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
            gas_params.ark_bls12_381_fr_eq * NumArgs::one()
        ),
        Some(Structure::BLS12381Fq12) => ark_eq_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            gas_params.ark_bls12_381_fq12_eq * NumArgs::one()
        ),
        Some(Structure::BLS12381G1) => ark_eq_internal!(
            context,
            args,
            ark_bls12_381::G1Projective,
            gas_params.ark_bls12_381_g1_proj_eq * NumArgs::one()
        ),
        Some(Structure::BLS12381G2) => ark_eq_internal!(
            context,
            args,
            ark_bls12_381::G2Projective,
            gas_params.ark_bls12_381_g2_proj_eq * NumArgs::one()
        ),
        Some(Structure::BLS12381Gt) => ark_eq_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            gas_params.ark_bls12_381_fq12_eq * NumArgs::one()
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
