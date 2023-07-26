// Copyright © Aptos Foundation

use crate::{
    abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        abort_invariant_violated, AlgebraContext, Structure, E_TOO_MUCH_MEMORY_USED,
        MEMORY_LIMIT_IN_BYTES, MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING,
        MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    safe_borrow_element, store_element, structure_from_ty_arg,
};
use aptos_gas_algebra::{Arg, GasExpression};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use aptos_types::on_chain_config::FeatureFlag;
use ark_ec::{pairing::Pairing, CurveGroup};
use move_core_types::gas_algebra::NumArgs;
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

fn feature_flag_of_pairing(
    g1_opt: Option<Structure>,
    g2_opt: Option<Structure>,
    gt_opt: Option<Structure>,
) -> Option<FeatureFlag> {
    match (g1_opt, g2_opt, gt_opt) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381G2), Some(Structure::BLS12381Gt)) => {
            Some(FeatureFlag::BLS12_381_STRUCTURES)
        },
        _ => None,
    }
}

macro_rules! abort_unless_pairing_enabled {
    ($context:ident, $g1_opt:expr, $g2_opt:expr, $gt_opt:expr) => {
        let flag_opt = feature_flag_of_pairing($g1_opt, $g2_opt, $gt_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

pub fn multi_pairing_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(3, ty_args.len());
    let g1_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let g2_opt = structure_from_ty_arg!(context, &ty_args[1]);
    let gt_opt = structure_from_ty_arg!(context, &ty_args[2]);
    abort_unless_pairing_enabled!(context, g1_opt, g2_opt, gt_opt);
    match (g1_opt, g2_opt, gt_opt) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381G2), Some(Structure::BLS12381Gt)) => {
            let g2_element_handles = safely_pop_arg!(args, Vec<u64>);
            let g1_element_handles = safely_pop_arg!(args, Vec<u64>);
            let num_entries = g1_element_handles.len();
            if num_entries != g2_element_handles.len() {
                return Err(SafeNativeError::Abort {
                    abort_code: MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING,
                });
            }

            context.charge(
                ALGEBRA_ARK_BLS12_381_G1_PROJ_TO_AFFINE.per::<Arg>()
                    * NumArgs::from(num_entries as u64),
            )?;
            let mut g1_elements_affine = Vec::with_capacity(num_entries);
            for handle in g1_element_handles {
                safe_borrow_element!(
                    context,
                    handle as usize,
                    ark_bls12_381::G1Projective,
                    ptr,
                    element
                );
                g1_elements_affine.push(element.into_affine());
            }

            context.charge(
                ALGEBRA_ARK_BLS12_381_G2_PROJ_TO_AFFINE.per::<Arg>()
                    * NumArgs::from(num_entries as u64),
            )?;
            let mut g2_elements_affine = Vec::with_capacity(num_entries);
            for handle in g2_element_handles {
                safe_borrow_element!(
                    context,
                    handle as usize,
                    ark_bls12_381::G2Projective,
                    ptr,
                    element
                );
                g2_elements_affine.push(element.into_affine());
            }

            context.charge(
                ALGEBRA_ARK_BLS12_381_MULTI_PAIRING_BASE
                    + ALGEBRA_ARK_BLS12_381_MULTI_PAIRING_PER_PAIR
                        * NumArgs::from(num_entries as u64),
            )?;
            let new_element =
                ark_bls12_381::Bls12_381::multi_pairing(g1_elements_affine, g2_elements_affine).0;
            let new_handle = store_element!(context, new_element)?;
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}

pub fn pairing_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(3, ty_args.len());
    let g1_opt = structure_from_ty_arg!(context, &ty_args[0]);
    let g2_opt = structure_from_ty_arg!(context, &ty_args[1]);
    let gt_opt = structure_from_ty_arg!(context, &ty_args[2]);
    abort_unless_pairing_enabled!(context, g1_opt, g2_opt, gt_opt);
    match (g1_opt, g2_opt, gt_opt) {
        (Some(Structure::BLS12381G1), Some(Structure::BLS12381G2), Some(Structure::BLS12381Gt)) => {
            let g2_element_handle = safely_pop_arg!(args, u64) as usize;
            let g1_element_handle = safely_pop_arg!(args, u64) as usize;
            safe_borrow_element!(
                context,
                g1_element_handle,
                ark_bls12_381::G1Projective,
                g1_element_ptr,
                g1_element
            );
            context.charge(ALGEBRA_ARK_BLS12_381_G1_PROJ_TO_AFFINE)?;
            let g1_element_affine = g1_element.into_affine();
            safe_borrow_element!(
                context,
                g2_element_handle,
                ark_bls12_381::G2Projective,
                g2_element_ptr,
                g2_element
            );
            context.charge(ALGEBRA_ARK_BLS12_381_G2_PROJ_TO_AFFINE)?;
            let g2_element_affine = g2_element.into_affine();
            context.charge(ALGEBRA_ARK_BLS12_381_PAIRING)?;
            let new_element =
                ark_bls12_381::Bls12_381::pairing(g1_element_affine, g2_element_affine).0;
            let new_handle = store_element!(context, new_element)?;
            Ok(smallvec![Value::u64(new_handle as u64)])
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
