// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[cfg(feature = "testing")]
use crate::{
    natives::cryptography::algebra::{
        AlgebraContext, Structure, BLS12381_GT_GENERATOR, BN254_GT_GENERATOR,
        E_RAND_BLS12381GT_GT_GENERATOR_LOADING_FAILED, E_RAND_BN254GT_GT_GENERATOR_LOADING_FAILED,
        E_RAND_INSECURE_NOT_IMPLEMENTED, E_TOO_MUCH_MEMORY_USED, MEMORY_LIMIT_IN_BYTES,
    },
    structure_from_ty_arg,
};
use aptos_native_interface::{SafeNativeContext, SafeNativeError, SafeNativeResult};
#[cfg(feature = "testing")]
use ark_ff::Field;
#[cfg(feature = "testing")]
use ark_std::{test_rng, UniformRand};
#[cfg(feature = "testing")]
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
#[cfg(feature = "testing")]
use smallvec::{smallvec, SmallVec};
#[cfg(feature = "testing")]
use std::{collections::VecDeque, rc::Rc};

macro_rules! store_element {
    ($context:expr, $obj:expr) => {{
        let context = &mut $context.extensions_mut().get_mut::<AlgebraContext>();
        let new_size = context.bytes_used + std::mem::size_of_val(&$obj);
        if new_size > MEMORY_LIMIT_IN_BYTES {
            Err(E_TOO_MUCH_MEMORY_USED)
        } else {
            let target_vec = &mut context.objs;
            context.bytes_used = new_size;
            let new_handle = target_vec.len();
            target_vec.push(Rc::new($obj));
            Ok(new_handle)
        }
    }};
}

#[cfg(feature = "testing")]
macro_rules! ark_rand_internal {
    ($context:expr, $typ:ty) => {{
        let element = <$typ>::rand(&mut test_rng());
        match store_element!($context, element) {
            Ok(new_handle) => Ok(smallvec![Value::u64(new_handle as u64)]),
            Err(abort_code) => Err(SafeNativeError::abort(abort_code)),
        }
    }};
}

#[cfg(feature = "testing")]
pub fn rand_insecure_internal(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
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
            let generator = BLS12381_GT_GENERATOR.as_ref().ok_or_else(|| {
                SafeNativeError::abort_with_message(
                    E_RAND_BLS12381GT_GT_GENERATOR_LOADING_FAILED,
                    "BLS12381 GT generator loading failed",
                )
            })?;
            let k = ark_bls12_381::Fr::rand(&mut test_rng());
            let k_bigint: ark_ff::BigInteger256 = k.into();
            let element = generator.pow(k_bigint);
            match store_element!(context, element) {
                Ok(handle) => Ok(smallvec![Value::u64(handle as u64)]),
                Err(abort_code) => Err(SafeNativeError::abort(abort_code)),
            }
        },
        Some(Structure::BN254Fr) => {
            ark_rand_internal!(context, ark_bn254::Fr)
        },
        Some(Structure::BN254Fq) => {
            ark_rand_internal!(context, ark_bn254::Fq)
        },
        Some(Structure::BN254Fq12) => {
            ark_rand_internal!(context, ark_bn254::Fq12)
        },
        Some(Structure::BN254G1) => {
            ark_rand_internal!(context, ark_bn254::G1Projective)
        },
        Some(Structure::BN254G2) => {
            ark_rand_internal!(context, ark_bn254::G2Projective)
        },
        Some(Structure::BN254Gt) => {
            let generator = BN254_GT_GENERATOR.as_ref().ok_or_else(|| {
                SafeNativeError::abort_with_message(
                    E_RAND_BN254GT_GT_GENERATOR_LOADING_FAILED,
                    "BN254 GT generator loading failed",
                )
            })?;
            let k = ark_bn254::Fr::rand(&mut test_rng());
            let k_bigint: ark_ff::BigInteger256 = k.into();
            let element = generator.pow(k_bigint);
            match store_element!(context, element) {
                Ok(handle) => Ok(smallvec![Value::u64(handle as u64)]),
                Err(abort_code) => Err(SafeNativeError::abort(abort_code)),
            }
        },
        _ => Err(SafeNativeError::abort_with_message(
            E_RAND_INSECURE_NOT_IMPLEMENTED,
            "Not implemented",
        )),
    }
}
