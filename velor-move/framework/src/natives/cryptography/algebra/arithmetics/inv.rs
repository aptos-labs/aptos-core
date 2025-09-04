// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abort_unless_arithmetics_enabled_for_structure, abort_unless_feature_flag_enabled,
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
use std::{collections::VecDeque, rc::Rc};

macro_rules! ark_inverse_internal {
    ($context:expr, $args:ident, $ark_typ:ty, $gas:expr) => {{
        let handle = safely_pop_arg!($args, u64) as usize;
        safe_borrow_element!($context, handle, $ark_typ, element_ptr, element);
        $context.charge($gas)?;
        match element.inverse() {
            Some(new_element) => {
                let new_handle = store_element!($context, new_element)?;
                Ok(smallvec![Value::bool(true), Value::u64(new_handle as u64)])
            },
            None => Ok(smallvec![Value::bool(false), Value::u64(0)]),
        }
    }};
}

pub fn inv_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_arithmetics_enabled_for_structure!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => ark_inverse_internal!(
            context,
            args,
            ark_bls12_381::Fr,
            ALGEBRA_ARK_BLS12_381_FR_INV
        ),
        Some(Structure::BLS12381Fq12) => ark_inverse_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            ALGEBRA_ARK_BLS12_381_FQ12_INV
        ),
        Some(Structure::BN254Fr) => {
            ark_inverse_internal!(context, args, ark_bn254::Fr, ALGEBRA_ARK_BN254_FR_INV)
        },
        Some(Structure::BN254Fq) => {
            ark_inverse_internal!(context, args, ark_bn254::Fq, ALGEBRA_ARK_BN254_FQ_INV)
        },
        Some(Structure::BN254Fq12) => {
            ark_inverse_internal!(context, args, ark_bn254::Fq12, ALGEBRA_ARK_BN254_FQ12_INV)
        },
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
