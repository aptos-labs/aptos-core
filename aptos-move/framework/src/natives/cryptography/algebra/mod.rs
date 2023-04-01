// Copyright © Aptos Foundation

// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "testing")]
use crate::natives::cryptography::algebra::rand::rand_insecure_internal;
#[cfg(feature = "testing")]
use crate::natives::helpers::make_test_only_native_from_func;
use crate::natives::{
    cryptography::algebra::{
        arithmetics::{
            add::add_internal, double::double_internal, mul::mul_internal, neg::neg_internal,
            sqr::sqr_internal, sub::sub_internal,
        },
        casting::{downcast_internal, upcast_internal},
        constants::{one_internal, order_internal, zero_internal},
        eq::eq_internal,
        gas::GasParameters,
        hash_to_structure::hash_to_internal,
        new::from_u64_internal,
        pairing::{multi_pairing_internal, pairing_internal},
        serialization::{deserialize_internal, serialize_internal},
    },
    helpers::make_safe_native,
};
use aptos_types::on_chain_config::{FeatureFlag, Features, TimedFeatures};
use arithmetics::{
    div::div_internal,
    inv::inv_internal,
    scalar_mul::{multi_scalar_mul_internal, scalar_mul_internal},
};
use ark_serialize::CanonicalDeserialize;
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMError;
use move_core_types::{language_storage::TypeTag, vm_status::StatusCode};
use move_vm_runtime::native_functions::NativeFunction;
use once_cell::sync::Lazy;
use std::{any::Any, hash::Hash, rc::Rc, sync::Arc};

pub mod arithmetics;
pub mod casting;
pub mod constants;
pub mod eq;
pub mod gas;
pub mod hash_to_structure;
pub mod new;
pub mod pairing;
#[cfg(feature = "testing")]
pub mod rand;
pub mod serialization;

/// Equivalent to `std::error::invalid_argument(0)` in Move.
const MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING: u64 = 0x01_0000;

/// Equivalent to `std::error::not_implemented(0)` in Move.
const MOVE_ABORT_CODE_NOT_IMPLEMENTED: u64 = 0x0C_0000;

/// This encodes an algebraic structure defined in `algebra_*.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum Structure {
    BLS12381Fq12,
    BLS12381G1Affine,
    BLS12381G2Affine,
    BLS12381Gt,
    BLS12381Fr,
}

impl TryFrom<TypeTag> for Structure {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_string().as_str() {
            "0x1::algebra_bls12381::Fr" => Ok(Structure::BLS12381Fr),
            "0x1::algebra_bls12381::Fq12" => Ok(Structure::BLS12381Fq12),
            "0x1::algebra_bls12381::G1Affine" => Ok(Structure::BLS12381G1Affine),
            "0x1::algebra_bls12381::G2Affine" => Ok(Structure::BLS12381G2Affine),
            "0x1::algebra_bls12381::Gt" => Ok(Structure::BLS12381Gt),
            _ => Err(()),
        }
    }
}

#[macro_export]
macro_rules! structure_from_ty_arg {
    ($context:expr, $typ:expr) => {{
        let type_tag = $context.type_to_type_tag($typ)?;
        Structure::try_from(type_tag).ok()
    }};
}

/// This encodes a supported serialization format defined in `algebra_*.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum SerializationFormat {
    BLS12381Fq12LscLsb,
    BLS12381G1AffineCompressed,
    BLS12381G1AffineUncompressed,
    BLS12381G2AffineCompressed,
    BLS12381G2AffineUncompressed,
    BLS12381Gt,
    BLS12381FrLsb,
    BLS12381FrMsb,
}

impl TryFrom<TypeTag> for SerializationFormat {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_string().as_str() {
            "0x1::algebra_bls12381::Fq12FormatLscLsb" => {
                Ok(SerializationFormat::BLS12381Fq12LscLsb)
            },
            "0x1::algebra_bls12381::G1AffineFormatUncompressed" => {
                Ok(SerializationFormat::BLS12381G1AffineUncompressed)
            },
            "0x1::algebra_bls12381::G1AffineFormatCompressed" => {
                Ok(SerializationFormat::BLS12381G1AffineCompressed)
            },
            "0x1::algebra_bls12381::G2AffineFormatUncompressed" => {
                Ok(SerializationFormat::BLS12381G2AffineUncompressed)
            },
            "0x1::algebra_bls12381::G2AffineFormatCompressed" => {
                Ok(SerializationFormat::BLS12381G2AffineCompressed)
            },
            "0x1::algebra_bls12381::GtFormat" => Ok(SerializationFormat::BLS12381Gt),
            "0x1::algebra_bls12381::FrFormatLsb" => Ok(SerializationFormat::BLS12381FrLsb),
            "0x1::algebra_bls12381::FrFormatMsb" => Ok(SerializationFormat::BLS12381FrMsb),
            _ => Err(()),
        }
    }
}

/// This encodes a supported hash-to-structure suite defined in `algebra_*.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum HashToStructureSuite {
    Bls12381g1XmdSha256SswuRo,
    Bls12381g2XmdSha256SswuRo,
}

impl TryFrom<TypeTag> for HashToStructureSuite {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_string().as_str() {
            "0x1::algebra_bls12381::H2SSuiteBls12381g1XmdSha256SswuRo" => {
                Ok(HashToStructureSuite::Bls12381g1XmdSha256SswuRo)
            },
            "0x1::algebra_bls12381::H2SSuiteBls12381g2XmdSha256SswuRo" => {
                Ok(HashToStructureSuite::Bls12381g2XmdSha256SswuRo)
            },
            _ => Err(()),
        }
    }
}

#[derive(Tid, Default)]
pub struct AlgebraContext {
    objs: Vec<Rc<dyn Any>>,
}

impl AlgebraContext {
    pub fn new() -> Self {
        Self { objs: Vec::new() }
    }
}

/// Try getting a pointer to the `handle`-th elements in `context` and assign it to a local variable `ptr_out`.
/// Then try casting it to a reference of `typ` and assign it in a local variable `ref_out`.
/// Abort the VM execution with invariant violation if anything above fails.
#[macro_export]
macro_rules! safe_borrow_element {
    ($context:expr, $handle:expr, $typ:ty, $ptr_out:ident, $ref_out:ident) => {
        let $ptr_out = $context
            .extensions()
            .get::<AlgebraContext>()
            .objs
            .get($handle)
            .ok_or_else(abort_invariant_violated)?
            .clone();
        let $ref_out = $ptr_out
            .downcast_ref::<$typ>()
            .ok_or_else(abort_invariant_violated)?;
    };
}

#[macro_export]
macro_rules! store_element {
    ($context:expr, $obj:expr) => {{
        let target_vec = &mut $context.extensions_mut().get_mut::<AlgebraContext>().objs;
        let ret = target_vec.len();
        target_vec.push(Rc::new($obj));
        ret
    }};
}

fn feature_flag_from_structure(structure_opt: Option<Structure>) -> Option<FeatureFlag> {
    match structure_opt {
        Some(Structure::BLS12381Fr)
        | Some(Structure::BLS12381Fq12)
        | Some(Structure::BLS12381G1Affine)
        | Some(Structure::BLS12381G2Affine)
        | Some(Structure::BLS12381Gt) => Some(FeatureFlag::BLS12_381_STRUCTURES),
        _ => None,
    }
}

#[macro_export]
macro_rules! abort_unless_arithmetics_enabled_for_structure {
    ($context:ident, $structure_opt:expr) => {
        let flag_opt = feature_flag_from_structure($structure_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

#[macro_export]
macro_rules! abort_unless_feature_flag_enabled {
    ($context:ident, $flag_opt:expr) => {
        match $flag_opt {
            Some(flag) if $context.get_feature_flags().is_enabled(flag) => {
                // Continue.
            },
            _ => {
                return Err(SafeNativeError::Abort {
                    abort_code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
                });
            },
        }
    };
}

fn abort_invariant_violated() -> PartialVMError {
    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
}

static BLS12381_GT_GENERATOR: Lazy<ark_bls12_381::Fq12> = Lazy::new(|| {
    let buf = hex::decode("b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f").unwrap();
    ark_bls12_381::Fq12::deserialize_uncompressed(buf.as_slice()).unwrap()
});

static BLS12381_R_LENDIAN: Lazy<Vec<u8>> = Lazy::new(|| {
    hex::decode("01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73").unwrap()
});

static BLS12381_R_SCALAR: Lazy<ark_ff::BigInteger256> = Lazy::new(|| {
    ark_ff::BigInteger256::deserialize_uncompressed(BLS12381_R_LENDIAN.as_slice()).unwrap()
});

pub fn make_all(
    gas_params: GasParameters,
    timed_features: TimedFeatures,
    features: Arc<Features>,
) -> impl Iterator<Item = (String, NativeFunction)> {
    let mut natives = vec![];

    // Always-on natives.
    natives.append(&mut vec![
        (
            "deserialize_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                deserialize_internal,
            ),
        ),
        (
            "downcast_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                downcast_internal,
            ),
        ),
        (
            "eq_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                eq_internal,
            ),
        ),
        (
            "add_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                add_internal,
            ),
        ),
        (
            "div_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                div_internal,
            ),
        ),
        (
            "inv_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                inv_internal,
            ),
        ),
        (
            "mul_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                mul_internal,
            ),
        ),
        (
            "neg_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                neg_internal,
            ),
        ),
        (
            "one_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                one_internal,
            ),
        ),
        (
            "sqr_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                sqr_internal,
            ),
        ),
        (
            "sub_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                sub_internal,
            ),
        ),
        (
            "zero_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                zero_internal,
            ),
        ),
        (
            "from_u64_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                from_u64_internal,
            ),
        ),
        (
            "double_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                double_internal,
            ),
        ),
        (
            "multi_scalar_mul_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                multi_scalar_mul_internal,
            ),
        ),
        (
            "order_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                order_internal,
            ),
        ),
        (
            "scalar_mul_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                scalar_mul_internal,
            ),
        ),
        (
            "hash_to_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                hash_to_internal,
            ),
        ),
        (
            "multi_pairing_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                multi_pairing_internal,
            ),
        ),
        (
            "pairing_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                pairing_internal,
            ),
        ),
        (
            "serialize_internal",
            make_safe_native(
                gas_params.clone(),
                timed_features.clone(),
                features.clone(),
                serialize_internal,
            ),
        ),
        (
            "upcast_internal",
            make_safe_native(gas_params, timed_features, features, upcast_internal),
        ),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.append(&mut vec![(
        "rand_insecure_internal",
        make_test_only_native_from_func(rand_insecure_internal),
    )]);

    crate::natives::helpers::make_module_natives(natives)
}
