// Copyright © Aptos Foundation

#[cfg(feature = "testing")]
use crate::{
    natives::cryptography::algebra::{AlgebraContext, Structure, BLS12381_GT_GENERATOR},
    store_element, structure_from_ty_arg,
};
#[cfg(feature = "testing")]
use ark_ff::Field;
#[cfg(feature = "testing")]
use ark_std::{test_rng, UniformRand};
use move_binary_format::errors::PartialVMResult;
use move_core_types::gas_algebra::InternalGas;
use move_vm_runtime::native_functions::NativeContext;
#[cfg(feature = "testing")]
use move_vm_types::{
    loaded_data::runtime_types::Type, natives::function::NativeResult, values::Value,
};
#[cfg(feature = "testing")]
use smallvec::smallvec;
#[cfg(feature = "testing")]
use std::{collections::VecDeque, rc::Rc};

#[cfg(feature = "testing")]
macro_rules! ark_rand_internal {
    ($context:expr, $typ:ty) => {{
        let element = <$typ>::rand(&mut test_rng());
        let handle = store_element!($context, element);
        Ok(NativeResult::ok(InternalGas::zero(), smallvec![
            Value::u64(handle as u64)
        ]))
    }};
}

#[cfg(feature = "testing")]
pub fn rand_insecure_internal(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut _args: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    assert_eq!(1, ty_args.len());
    let structure_opt = structure_from_ty_arg!(context, &ty_args[0]);
    match structure_opt {
        Some(Structure::BLS12381Fr) => {
            ark_rand_internal!(context, ark_bls12_381::Fr)
        },
        Some(Structure::BLS12381Fq12) => {
            ark_rand_internal!(context, ark_bls12_381::Fq12)
        },
        Some(Structure::BLS12381G1) => {
            ark_rand_internal!(context, ark_bls12_381::G1Projective)
        },
        Some(Structure::BLS12381G2) => {
            ark_rand_internal!(context, ark_bls12_381::G2Projective)
        },
        Some(Structure::BLS12381Gt) => {
            let k = ark_bls12_381::Fr::rand(&mut test_rng());
            let k_bigint: ark_ff::BigInteger256 = k.into();
            let element = BLS12381_GT_GENERATOR.pow(k_bigint);
            let handle = store_element!(context, element);
            Ok(NativeResult::ok(InternalGas::zero(), smallvec![
                Value::u64(handle as u64)
            ]))
        },
        _ => unreachable!(),
    }
}
