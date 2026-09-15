// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#[cfg(feature = "testing")]
use crate::cryptography::algebra::rand::rand_insecure_internal;
use crate::cryptography::algebra::{
    arithmetics::{
        add::add_internal, double::double_internal, mul::mul_internal, neg::neg_internal,
        sqr::sqr_internal, sub::sub_internal,
    },
    casting::{downcast_internal, upcast_internal},
    constants::{one_internal, order_internal, zero_internal},
    eq::eq_internal,
    hash_to_structure::hash_to_internal,
    new::from_u64_internal,
    pairing::{multi_pairing_internal, pairing_internal},
    serialization::{deserialize_internal, serialize_internal},
};
use aptos_native_interface::{RawSafeNative, SafeNativeBuilder};
pub(crate) use aptos_types::crypto::algebra::{
    BLS12381_GT_GENERATOR, BLS12381_Q12_LENDIAN, BLS12381_R_LENDIAN, BLS12381_R_SCALAR,
    BN254_GT_GENERATOR, BN254_Q12_LENDIAN, BN254_Q_LENDIAN, BN254_R_LENDIAN, BN254_R_SCALAR,
};
use aptos_types::on_chain_config::FeatureFlag;
use arithmetics::{
    div::div_internal,
    inv::inv_internal,
    scalar_mul::{multi_scalar_mul_internal, scalar_mul_internal},
};
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMError;
use move_core_types::{language_storage::TypeTag, vm_status::StatusCode};
use move_vm_runtime::{
    native_extensions::{NativeRuntimeRefCheckModelsCompleted, SessionListener},
    native_functions::NativeFunction,
};
use std::{any::Any, hash::Hash, rc::Rc};

pub mod arithmetics;
pub mod casting;
pub mod constants;
pub mod eq;
pub mod hash_to_structure;
pub mod new;
pub mod pairing;
#[cfg(feature = "testing")]
pub mod rand;
pub mod serialization;

/// Equivalent to `std::error::invalid_argument(0)` in Move.
const MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING: u64 = 0x01_0002;

/// Equivalent to `std::error::not_implemented(0)` in Move.
const MOVE_ABORT_CODE_NOT_IMPLEMENTED: u64 = 0x0C_0001;

pub(crate) const E_CONSTANTS_BLS12381GT_GT_GENERATOR_LOADING_FAILED: u64 = 0x0A_0001;
pub(crate) const E_CONSTANTS_BN254GT_GT_GENERATOR_LOADING_FAILED: u64 = 0x0A_0002;
pub(crate) const E_CONSTANTS_BLS12381_R_ORDER_LOADING_FAILED: u64 = 0x0A_0003;
pub(crate) const E_CONSTANTS_BLS12381FQ12_Q12_ORDER_LOADING_FAILED: u64 = 0x0A_0004;
pub(crate) const E_CONSTANTS_BN254FQ12_Q12_ORDER_LOADING_FAILED: u64 = 0x0A_0005;
pub(crate) const E_CASTING_BLS12381_R_SCALAR_LOADING_FAILED: u64 = 0x0A_0006;
pub(crate) const E_SERIALIZATION_BLS12381GT_CONST_LOADING_FAILED: u64 = 0x0A_0007;
#[cfg(feature = "testing")]
pub(crate) const E_RAND_BLS12381GT_GT_GENERATOR_LOADING_FAILED: u64 = 0x0A_0008;
#[cfg(feature = "testing")]
pub(crate) const E_RAND_BN254GT_GT_GENERATOR_LOADING_FAILED: u64 = 0x0A_0009;
pub(crate) const E_SCALAR_MUL_MSM_WINDOW_SIZE_FAILED: u64 = 0x0A_000A;
pub(crate) const E_SCALAR_MUL_MSM_COMPUTATION_FAILED: u64 = 0x0A_000B;
pub(crate) const E_HASH_TO_STRUCTURE_BLS12381G1_MAPPER_FAILED: u64 = 0x0A_000C;
pub(crate) const E_HASH_TO_STRUCTURE_BLS12381G1_HASH_FAILED: u64 = 0x0A_000D;
pub(crate) const E_HASH_TO_STRUCTURE_BLS12381G2_MAPPER_FAILED: u64 = 0x0A_000E;
pub(crate) const E_HASH_TO_STRUCTURE_BLS12381G2_HASH_FAILED: u64 = 0x0A_000F;
#[cfg(feature = "testing")]
pub(crate) const E_RAND_INSECURE_NOT_IMPLEMENTED: u64 = 0x0A_0010;

/// This encodes an algebraic structure defined in `*_algebra.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum Structure {
    BLS12381Fq12,
    BLS12381G1,
    BLS12381G2,
    BLS12381Gt,
    BLS12381Fr,

    BN254Fr,
    BN254Fq,
    BN254Fq12,
    BN254G1,
    BN254G2,
    BN254Gt,
}

impl TryFrom<TypeTag> for Structure {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_canonical_string().as_str() {
            "0x1::bls12381_algebra::Fr" => Ok(Structure::BLS12381Fr),
            "0x1::bls12381_algebra::Fq12" => Ok(Structure::BLS12381Fq12),
            "0x1::bls12381_algebra::G1" => Ok(Structure::BLS12381G1),
            "0x1::bls12381_algebra::G2" => Ok(Structure::BLS12381G2),
            "0x1::bls12381_algebra::Gt" => Ok(Structure::BLS12381Gt),

            "0x1::bn254_algebra::Fr" => Ok(Self::BN254Fr),
            "0x1::bn254_algebra::Fq" => Ok(Self::BN254Fq),
            "0x1::bn254_algebra::Fq12" => Ok(Self::BN254Fq12),
            "0x1::bn254_algebra::G1" => Ok(Self::BN254G1),
            "0x1::bn254_algebra::G2" => Ok(Self::BN254G2),
            "0x1::bn254_algebra::Gt" => Ok(Self::BN254Gt),
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

/// This encodes a supported serialization format defined in `*_algebra.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum SerializationFormat {
    BLS12381Fq12LscLsb,
    BLS12381G1Compressed,
    BLS12381G1Uncompressed,
    BLS12381G2Compressed,
    BLS12381G2Uncompressed,
    BLS12381Gt,
    BLS12381FrLsb,
    BLS12381FrMsb,

    BN254G1Compressed,
    BN254G1Uncompressed,
    BN254G2Compressed,
    BN254G2Uncompressed,
    BN254Gt,
    BN254FrLsb,
    BN254FrMsb,
    BN254FqLsb,
    BN254FqMsb,
    BN254Fq12LscLsb,
}

impl TryFrom<TypeTag> for SerializationFormat {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_canonical_string().as_str() {
            "0x1::bls12381_algebra::FormatFq12LscLsb" => {
                Ok(SerializationFormat::BLS12381Fq12LscLsb)
            },
            "0x1::bls12381_algebra::FormatG1Uncompr" => {
                Ok(SerializationFormat::BLS12381G1Uncompressed)
            },
            "0x1::bls12381_algebra::FormatG1Compr" => Ok(SerializationFormat::BLS12381G1Compressed),
            "0x1::bls12381_algebra::FormatG2Uncompr" => {
                Ok(SerializationFormat::BLS12381G2Uncompressed)
            },
            "0x1::bls12381_algebra::FormatG2Compr" => Ok(SerializationFormat::BLS12381G2Compressed),
            "0x1::bls12381_algebra::FormatGt" => Ok(SerializationFormat::BLS12381Gt),
            "0x1::bls12381_algebra::FormatFrLsb" => Ok(SerializationFormat::BLS12381FrLsb),
            "0x1::bls12381_algebra::FormatFrMsb" => Ok(SerializationFormat::BLS12381FrMsb),

            "0x1::bn254_algebra::FormatG1Uncompr" => Ok(Self::BN254G1Uncompressed),
            "0x1::bn254_algebra::FormatG1Compr" => Ok(Self::BN254G1Compressed),
            "0x1::bn254_algebra::FormatG2Uncompr" => Ok(Self::BN254G2Uncompressed),
            "0x1::bn254_algebra::FormatG2Compr" => Ok(Self::BN254G2Compressed),
            "0x1::bn254_algebra::FormatGt" => Ok(Self::BN254Gt),
            "0x1::bn254_algebra::FormatFrLsb" => Ok(Self::BN254FrLsb),
            "0x1::bn254_algebra::FormatFrMsb" => Ok(Self::BN254FrMsb),
            "0x1::bn254_algebra::FormatFqLsb" => Ok(Self::BN254FqLsb),
            "0x1::bn254_algebra::FormatFqMsb" => Ok(Self::BN254FqMsb),
            "0x1::bn254_algebra::FormatFq12LscLsb" => Ok(Self::BN254Fq12LscLsb),
            _ => Err(()),
        }
    }
}

/// This encodes a supported hash-to-structure suite defined in `*_algebra.move`.
#[derive(Copy, Clone, Eq, Hash, PartialEq)]
pub enum HashToStructureSuite {
    Bls12381g1XmdSha256SswuRo,
    Bls12381g2XmdSha256SswuRo,
}

impl TryFrom<TypeTag> for HashToStructureSuite {
    type Error = ();

    fn try_from(value: TypeTag) -> Result<Self, Self::Error> {
        match value.to_canonical_string().as_str() {
            "0x1::bls12381_algebra::HashG1XmdSha256SswuRo" => {
                Ok(HashToStructureSuite::Bls12381g1XmdSha256SswuRo)
            },
            "0x1::bls12381_algebra::HashG2XmdSha256SswuRo" => {
                Ok(HashToStructureSuite::Bls12381g2XmdSha256SswuRo)
            },
            _ => Err(()),
        }
    }
}

/// This limit ensures that no more than 1MB will be allocated for elements per VM session.
const MEMORY_LIMIT_IN_BYTES: usize = 1 << 20;

/// Equivalent to `std::error::resource_exhausted(3)` in Move.
const E_TOO_MUCH_MEMORY_USED: u64 = 0x09_0003;

#[derive(Tid, Default)]
pub struct AlgebraContext {
    bytes_used: usize,
    objs: Vec<Rc<dyn Any>>,
}

impl SessionListener for AlgebraContext {
    fn start(&mut self, _session_hash: &[u8; 32], _script_hash: &[u8], _session_counter: u8) {
        self.bytes_used = 0;
        self.objs.clear();
    }

    fn finish(&mut self) {
        // No state changes to save.
    }

    fn abort(&mut self) {
        // No state changes to abort. Context will be reset on new session's start.
    }
}

impl NativeRuntimeRefCheckModelsCompleted for AlgebraContext {
    // No native functions in this context return references, so no models to add.
}

impl AlgebraContext {
    pub fn new() -> Self {
        Self {
            bytes_used: 0,
            objs: Vec::new(),
        }
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
        let context = &mut $context.extensions_mut().get_mut::<AlgebraContext>();
        let new_size = context.bytes_used + std::mem::size_of_val(&$obj);
        if new_size > MEMORY_LIMIT_IN_BYTES {
            Err(SafeNativeError::abort_with_message(
                E_TOO_MUCH_MEMORY_USED,
                format!(
                    "Algebra context memory {}-byte limit exceeded: currently using {} bytes; was asked for {} bytes",
                    MEMORY_LIMIT_IN_BYTES, context.bytes_used, new_size,
                ),
            ))
        } else {
            let target_vec = &mut context.objs;
            context.bytes_used = new_size;
            let ret = target_vec.len();
            target_vec.push(Rc::new($obj));
            Ok(ret)
        }
    }};
}

fn feature_flag_from_structure(structure_opt: Option<Structure>) -> Option<FeatureFlag> {
    match structure_opt {
        Some(Structure::BLS12381Fr)
        | Some(Structure::BLS12381Fq12)
        | Some(Structure::BLS12381G1)
        | Some(Structure::BLS12381G2)
        | Some(Structure::BLS12381Gt) => Some(FeatureFlag::BLS12_381_STRUCTURES),
        Some(Structure::BN254Fr)
        | Some(Structure::BN254Fq)
        | Some(Structure::BN254Fq12)
        | Some(Structure::BN254G1)
        | Some(Structure::BN254G2)
        | Some(Structure::BN254Gt) => Some(FeatureFlag::BN254_STRUCTURES),
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
                return Err(SafeNativeError::abort(MOVE_ABORT_CODE_NOT_IMPLEMENTED));
            },
        }
    };
}

fn abort_invariant_violated() -> PartialVMError {
    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
        .with_message("aptos_std::crypto_algebra native abort".to_string())
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let mut natives = vec![];

    natives.extend([
        (
            "deserialize_internal",
            deserialize_internal as RawSafeNative,
        ),
        ("downcast_internal", downcast_internal),
        ("eq_internal", eq_internal),
        ("add_internal", add_internal),
        ("div_internal", div_internal),
        ("inv_internal", inv_internal),
        ("mul_internal", mul_internal),
        ("neg_internal", neg_internal),
        ("one_internal", one_internal),
        ("sqr_internal", sqr_internal),
        ("sub_internal", sub_internal),
        ("zero_internal", zero_internal),
        ("from_u64_internal", from_u64_internal),
        ("double_internal", double_internal),
        ("multi_scalar_mul_internal", multi_scalar_mul_internal),
        ("order_internal", order_internal),
        ("scalar_mul_internal", scalar_mul_internal),
        ("hash_to_internal", hash_to_internal),
        ("multi_pairing_internal", multi_pairing_internal),
        ("pairing_internal", pairing_internal),
        ("serialize_internal", serialize_internal),
        ("upcast_internal", upcast_internal),
    ]);

    // Test-only natives.
    #[cfg(feature = "testing")]
    natives.extend([(
        "rand_insecure_internal",
        rand_insecure_internal as RawSafeNative,
    )]);

    builder.make_named_natives(natives)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_extension_update() {
        let mut ctx = AlgebraContext::new();
        ctx.bytes_used = 20;
        ctx.objs.push(Rc::new("something".to_string()));
        ctx.start(&[0; 32], &[], 0);

        let AlgebraContext { bytes_used, objs } = ctx;
        assert_eq!(bytes_used, 0);
        assert!(objs.is_empty());
    }
}
