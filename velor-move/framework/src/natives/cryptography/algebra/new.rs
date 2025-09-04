// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    abort_unless_arithmetics_enabled_for_structure, abort_unless_feature_flag_enabled,
    natives::cryptography::algebra::{
        feature_flag_from_structure, AlgebraContext, Structure, E_TOO_MUCH_MEMORY_USED,
        MEMORY_LIMIT_IN_BYTES, MOVE_ABORT_CODE_NOT_IMPLEMENTED,
    },
    store_element, structure_from_ty_arg,
};
use velor_gas_schedule::gas_params::natives::velor_framework::*;
use velor_native_interface::{
    safely_pop_arg, SafeNativeContext, SafeNativeError, SafeNativeResult,
};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{collections::VecDeque, rc::Rc};

macro_rules! from_u64_internal {
    ($context:expr, $args:ident, $typ:ty, $gas:expr) => {{
        let value = safely_pop_arg!($args, u64);
        $context.charge($gas)?;
        let element = <$typ>::from(value as u64);
        let handle = store_element!($context, element)?;
        Ok(smallvec![Value::u64(handle as u64)])
    }};
}

pub fn from_u64_internal(
    context: &mut SafeNativeContext,
    ty_args: Vec<Type>,
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    abort_unless_arithmetics_enabled_for_structure!(context, structure_opt);
    match structure_opt {
        Some(Structure::BLS12381Fr) => from_u64_internal!(
            context,
            args,
            ark_bls12_381::Fr,
            ALGEBRA_ARK_BLS12_381_FR_FROM_U64
        ),
        Some(Structure::BLS12381Fq12) => from_u64_internal!(
            context,
            args,
            ark_bls12_381::Fq12,
            ALGEBRA_ARK_BLS12_381_FQ12_FROM_U64
        ),
        Some(Structure::BN254Fr) => {
            from_u64_internal!(context, args, ark_bn254::Fr, ALGEBRA_ARK_BN254_FR_FROM_U64)
        },
        Some(Structure::BN254Fq) => {
            from_u64_internal!(context, args, ark_bn254::Fq, ALGEBRA_ARK_BN254_FQ_FROM_U64)
        },
        Some(Structure::BN254Fq12) => from_u64_internal!(
            context,
            args,
            ark_bn254::Fq12,
            ALGEBRA_ARK_BN254_FQ12_FROM_U64
        ),
        _ => Err(SafeNativeError::Abort {
            abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        }),
    }
}
