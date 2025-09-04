// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

#[cfg(feature = "testing")]
use crate::natives::cryptography::algebra::rand::rand_insecure_internal;
use crate::natives::cryptography::algebra::{
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
use aptos_types::on_chain_config::FeatureFlag;
use arithmetics::{
    div::div_internal,
    inv::inv_internal,
    scalar_mul::{multi_scalar_mul_internal, scalar_mul_internal},
};
use ark_ff::{BigInteger, PrimeField};
use ark_serialize::CanonicalDeserialize;
use better_any::{Tid, TidAble};
use move_binary_format::errors::PartialVMError;
use move_core_types::{language_storage::TypeTag, vm_status::StatusCode};
use move_vm_runtime::native_functions::NativeFunction;
use once_cell::sync::Lazy;
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
    ($context:expr_2021, $typ:expr_2021) => {{
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
    ($context:expr_2021, $handle:expr_2021, $typ:ty, $ptr_out:ident, $ref_out:ident) => {
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
    ($context:expr_2021, $obj:expr_2021) => {{
        let context = &mut $context.extensions_mut().get_mut::<AlgebraContext>();
        let new_size = context.bytes_used + std::mem::size_of_val(&$obj);
        if new_size > MEMORY_LIMIT_IN_BYTES {
            Err(SafeNativeError::Abort {
                abort_code: E_TOO_MUCH_MEMORY_USED,
            })
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
    ($context:ident, $structure_opt:expr_2021) => {
        let flag_opt = feature_flag_from_structure($structure_opt);
        abort_unless_feature_flag_enabled!($context, flag_opt);
    };
}

#[macro_export]
macro_rules! abort_unless_feature_flag_enabled {
    ($context:ident, $flag_opt:expr_2021) => {
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
        .with_message("aptos_std::crypto_algebra native abort".to_string())
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

static BLS12381_Q12_LENDIAN: Lazy<Vec<u8>> = Lazy::new(|| {
    hex::decode("1175f55da544c7625f8ccb1360e2b1d3ca40747811c8f5ed04440afe232b476c0215676aec05f2a44ac2da6b6d1b7cff075e7b2a587e0aab601a8d3db4f0d29906e5e4d0d78119f396d5a59f0f8d1ca8bca62540be6ab9c12d0ca00de1f311f106278d000e55a393c9766a74e0d08a298450f60d7e666575e3354bf14b8731f4e721c0c180a5ed55c2f8f51f815baecbf96b5fc717eb58ac161a27d1d5f2bdc1a079609b9d6449165b2466b32a01eac7992a1ea0cac2f223cde1d56f9bbccc67afe44621daf858df3fc0eb837818f3e42ab3e131ce4e492efa63c108e6ef91c29ed63b3045baebcb0ab8d203c7f558beaffccba31b12aca7f54b58d0c28340e4fdb3c7c94fe9c4fef9d640ff2fcff02f1748416cbed0981fbff49f0e39eaf8a30273e67ed851944d33d6a593ef5ddcd62da84568822a6045b633bf6a513b3cfe8f9de13e76f8dcbd915980dec205eab6a5c0c72dcebd9afff1d25509ddbf33f8e24131fbd74cda93336514340cf8036b66b09ed9e6a6ac37e22fb3ac407e321beae8cd9fe74c8aaeb4edaa9a7272848fc623f6fe835a2e647379f547fc5ec6371318a85bfa60009cb20ccbb8a467492988a87633c14c0324ba0d0c3e1798ed29c8494cea35023746da05e35d184b4a301d5b2238d665495c6318b5af8653758008952d06cb9e62487b196d64383c73c06d6e1cccdf9b3ce8f95679e7050d949004a55f4ccf95b2552880ae36d1f7e09504d2338316d87d14a064511a295d768113e301bdf9d4383a8be32192d3f2f3b2de14181c73839a7cb4af5301").unwrap()
});

static BN254_GT_GENERATOR: Lazy<ark_bn254::Fq12> = Lazy::new(|| {
    // Gt generator is defined as the `e(g1_generator, g2_generator)`.
    let buf = hex::decode("950e879d73631f5eb5788589eb5f7ef8d63e0a28de1ba00dfe4ca9ed3f252b264a8afb8eb4349db466ed1809ea4d7c39bdab7938821f1b0a00a295c72c2de002e01dbdfd0254134efcb1ec877395d25f937719b344adb1a58d129be2d6f2a9132b16a16e8ab030b130e69c69bd20b4c45986e6744a98314b5c1a0f50faa90b04dbaf9ef8aeeee3f50be31c210b598f4752f073987f9d35be8f6770d83f2ffc0af0d18dd9d2dbcdf943825acc12a7a9ddca45e629d962c6bd64908c3930a5541cfe2924dcc5580d5cef7a4bfdec90a91b59926f850d4a7923c01a5a5dbf0f5c094a2b9fb9d415820fa6b40c59bb9eade9c953407b0fc11da350a9d872cad6d3142974ca385854afdf5f583c04231adc5957c8914b6b20dc89660ed7c3bbe7c01d972be2d53ecdb27a1bcc16ac610db95aa7d237c8ff55a898cb88645a0e32530b23d7ebf5dafdd79b0f9c2ac4ba07ce18d3d16cf36e47916c4cae5d08d3afa813972c769e8514533e380c9443b3e1ee5c96fa3a0a73f301b626454721527bf900").unwrap();
    ark_bn254::Fq12::deserialize_uncompressed(buf.as_slice()).unwrap()
});

static BN254_R_LENDIAN: Lazy<Vec<u8>> = Lazy::new(|| BN254_R_SCALAR.to_bytes_le());
const BN254_R_SCALAR: ark_ff::BigInteger256 = ark_bn254::Fr::MODULUS;
static BN254_Q_LENDIAN: Lazy<Vec<u8>> = Lazy::new(|| BN254_Q_SCALAR.to_bytes_le());
const BN254_Q_SCALAR: ark_ff::BigInteger256 = ark_bn254::Fq::MODULUS;

/// generated by: ark_bn254::Fq::MODULUS.pow(12)
static BN254_Q12_LENDIAN: Lazy<Vec<u8>> = Lazy::new(|| {
    hex::decode("21f186cad2e2d4c1dbaf8a066b0ebf41f734e3f859b1c523a6c1f4d457413fdbe3cd44add090135d3ae519acc30ee3bdb6bfac6573b767e975b18a77d53cdcddebf3672c74da9d1409d51b2b2db7ff000d59e3aa7cf09220159f925c86b65459ca6558c4eaa703bf45d85030ff85cc6a879c7e2c4034f7045faf20e4d3dcfffac5eb6634c3e7b939b69b2be70bdf6b9a4680297839b4e3a48cd746bd4d0ea82749ffb7e71bd9b3fb10aa684d71e6adab1250b1d8604d91b51c76c256a50b60ddba2f52b6cc853ac926c6ea86d09d400b2f2330e5c8e92e38905ba50a50c9e11cd979c284bf1327ccdc051a6da1a4a7eac5cec16757a27a1a2311bedd108a9b21ac0814269e7523a5dd3a1f5f4767ffe504a6cb3994fb0ec98d5cd5da00b9cb1188a85f2aa871ecb8a0f9d64141f1ccd2699c138e0ef9ac4d8d6a692b29db0f38b60eb08426ab46109fbab9a5221bb44dd338aafebcc4e6c10dd933597f3ff44ba41d04e82871447f3a759cfa9397c22c0c77f13618dfb65adc8aacf008").unwrap()
});

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
