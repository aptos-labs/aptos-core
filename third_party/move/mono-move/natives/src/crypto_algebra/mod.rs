// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for the `crypto_algebra` module (`aptos_std::crypto_algebra`).
//!
//! An algebraic element never crosses the Move/native boundary. Each element
//! lives in a per-transaction Rust store ([`AlgebraStore`]) and Move holds a
//! `u64` handle into it. Which concrete arkworks type a handle denotes is
//! decided by the native's type arguments: the marker structs declared in
//! `bls12381_algebra.move` and `bn254_algebra.move` carry no data and exist
//! only to select an implementation.
//!
//! Every native here is therefore polymorphic, and resolves its markers in the
//! body through [`structure_of`], [`format_of`] and [`suite_of`].
//
// TODO(correctness): the legacy VM wraps `pairing`, `multi_pairing`,
// `scalar_mul` and `multi_scalar_mul` in `aptos_native_interface`'s
// `with_native_rayon`. Arkworks spawns rayon work, and without an isolated pool
// a native running on a Block-STM worker can deadlock. Results are unaffected,
// so parity holds; this must clear before MonoMove runs on a real worker pool.

mod arithmetic;
mod casting;
mod constants;
mod hash_to;
mod pairing;
#[cfg(feature = "testing")]
mod rand;
mod scalar_mul;
mod serialization;

use crate::{polymorphic_natives, NativeEntry};
use ark_ff::{BigInteger, PrimeField};
use ark_serialize::CanonicalDeserialize;
use mono_move_core::{
    native::{native_invariant_violation, NativeContextFamily, NativeExtension, NativeStatus},
    types::{view_name, view_type, view_type_list, InternedType, Type},
    VMResult,
};
use move_core_types::account_address::AccountAddress;
use once_cell::sync::Lazy;
use std::any::Any;

/// Equivalent to `std::error::invalid_argument(0)` in Move.
const MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING: u64 = 0x01_0002;

/// Equivalent to `std::error::not_implemented(0)` in Move.
const MOVE_ABORT_CODE_NOT_IMPLEMENTED: u64 = 0x0C_0001;

/// Equivalent to `std::error::resource_exhausted(3)` in Move.
const E_TOO_MUCH_MEMORY_USED: u64 = 0x09_0003;

const E_CONSTANTS_BLS12381GT_GT_GENERATOR_LOADING_FAILED: u64 = 0x0A_0001;
const E_CONSTANTS_BN254GT_GT_GENERATOR_LOADING_FAILED: u64 = 0x0A_0002;
const E_CONSTANTS_BLS12381_R_ORDER_LOADING_FAILED: u64 = 0x0A_0003;
const E_CONSTANTS_BLS12381FQ12_Q12_ORDER_LOADING_FAILED: u64 = 0x0A_0004;
const E_CONSTANTS_BN254FQ12_Q12_ORDER_LOADING_FAILED: u64 = 0x0A_0005;
const E_CASTING_BLS12381_R_SCALAR_LOADING_FAILED: u64 = 0x0A_0006;
const E_SERIALIZATION_BLS12381GT_CONST_LOADING_FAILED: u64 = 0x0A_0007;
#[cfg(feature = "testing")]
const E_RAND_BLS12381GT_GT_GENERATOR_LOADING_FAILED: u64 = 0x0A_0008;
#[cfg(feature = "testing")]
const E_RAND_BN254GT_GT_GENERATOR_LOADING_FAILED: u64 = 0x0A_0009;
const E_SCALAR_MUL_MSM_COMPUTATION_FAILED: u64 = 0x0A_000B;
const E_HASH_TO_STRUCTURE_BLS12381G1_MAPPER_FAILED: u64 = 0x0A_000C;
const E_HASH_TO_STRUCTURE_BLS12381G1_HASH_FAILED: u64 = 0x0A_000D;
const E_HASH_TO_STRUCTURE_BLS12381G2_MAPPER_FAILED: u64 = 0x0A_000E;
const E_HASH_TO_STRUCTURE_BLS12381G2_HASH_FAILED: u64 = 0x0A_000F;
#[cfg(feature = "testing")]
const E_RAND_INSECURE_NOT_IMPLEMENTED: u64 = 0x0A_0010;

/// An algebraic structure defined in `*_algebra.move`.
#[derive(Copy, Clone, Eq, PartialEq)]
enum Structure {
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

/// A serialization format defined in `*_algebra.move`.
#[derive(Copy, Clone, Eq, PartialEq)]
enum SerializationFormat {
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

/// A hash-to-structure suite defined in `*_algebra.move`.
#[derive(Copy, Clone, Eq, PartialEq)]
enum HashToStructureSuite {
    Bls12381g1XmdSha256SswuRo,
    Bls12381g2XmdSha256SswuRo,
}

/// Splits `ty` into its module and struct name, or returns [`None`] if it is
/// not a marker struct declared at `0x1`.
///
/// The empty type-argument check is parity, not paranoia: the legacy VM matches
/// the canonical type-tag string, which carries `<..>` for an instantiated
/// struct and so never matches a marker name.
//
// The legacy VM can also fail before matching, when building the type tag of an
// oversized type. MonoMove reads the interned type directly and builds no tag,
// so that path does not exist here.
fn marker_of(ty: InternedType) -> Option<(&'static str, &'static str)> {
    let Type::Nominal {
        module_id,
        name,
        ty_args,
    } = view_type(ty)
    else {
        return None;
    };
    if !view_type_list(*ty_args).is_empty() {
        return None;
    }
    // SAFETY: interned ids are valid for the executable's lifetime.
    let module_id = unsafe { module_id.as_ref_unchecked() };
    if module_id.address() != &AccountAddress::ONE {
        return None;
    }
    Some((view_name(module_id.name()), view_name(*name)))
}

fn structure_of(ty: InternedType) -> Option<Structure> {
    match marker_of(ty)? {
        ("bls12381_algebra", "Fr") => Some(Structure::BLS12381Fr),
        ("bls12381_algebra", "Fq12") => Some(Structure::BLS12381Fq12),
        ("bls12381_algebra", "G1") => Some(Structure::BLS12381G1),
        ("bls12381_algebra", "G2") => Some(Structure::BLS12381G2),
        ("bls12381_algebra", "Gt") => Some(Structure::BLS12381Gt),

        ("bn254_algebra", "Fr") => Some(Structure::BN254Fr),
        ("bn254_algebra", "Fq") => Some(Structure::BN254Fq),
        ("bn254_algebra", "Fq12") => Some(Structure::BN254Fq12),
        ("bn254_algebra", "G1") => Some(Structure::BN254G1),
        ("bn254_algebra", "G2") => Some(Structure::BN254G2),
        ("bn254_algebra", "Gt") => Some(Structure::BN254Gt),
        _ => None,
    }
}

fn format_of(ty: InternedType) -> Option<SerializationFormat> {
    match marker_of(ty)? {
        ("bls12381_algebra", "FormatFq12LscLsb") => Some(SerializationFormat::BLS12381Fq12LscLsb),
        ("bls12381_algebra", "FormatG1Uncompr") => {
            Some(SerializationFormat::BLS12381G1Uncompressed)
        },
        ("bls12381_algebra", "FormatG1Compr") => Some(SerializationFormat::BLS12381G1Compressed),
        ("bls12381_algebra", "FormatG2Uncompr") => {
            Some(SerializationFormat::BLS12381G2Uncompressed)
        },
        ("bls12381_algebra", "FormatG2Compr") => Some(SerializationFormat::BLS12381G2Compressed),
        ("bls12381_algebra", "FormatGt") => Some(SerializationFormat::BLS12381Gt),
        ("bls12381_algebra", "FormatFrLsb") => Some(SerializationFormat::BLS12381FrLsb),
        ("bls12381_algebra", "FormatFrMsb") => Some(SerializationFormat::BLS12381FrMsb),

        ("bn254_algebra", "FormatG1Uncompr") => Some(SerializationFormat::BN254G1Uncompressed),
        ("bn254_algebra", "FormatG1Compr") => Some(SerializationFormat::BN254G1Compressed),
        ("bn254_algebra", "FormatG2Uncompr") => Some(SerializationFormat::BN254G2Uncompressed),
        ("bn254_algebra", "FormatG2Compr") => Some(SerializationFormat::BN254G2Compressed),
        ("bn254_algebra", "FormatGt") => Some(SerializationFormat::BN254Gt),
        ("bn254_algebra", "FormatFrLsb") => Some(SerializationFormat::BN254FrLsb),
        ("bn254_algebra", "FormatFrMsb") => Some(SerializationFormat::BN254FrMsb),
        ("bn254_algebra", "FormatFqLsb") => Some(SerializationFormat::BN254FqLsb),
        ("bn254_algebra", "FormatFqMsb") => Some(SerializationFormat::BN254FqMsb),
        ("bn254_algebra", "FormatFq12LscLsb") => Some(SerializationFormat::BN254Fq12LscLsb),
        _ => None,
    }
}

fn suite_of(ty: InternedType) -> Option<HashToStructureSuite> {
    match marker_of(ty)? {
        ("bls12381_algebra", "HashG1XmdSha256SswuRo") => {
            Some(HashToStructureSuite::Bls12381g1XmdSha256SswuRo)
        },
        ("bls12381_algebra", "HashG2XmdSha256SswuRo") => {
            Some(HashToStructureSuite::Bls12381g2XmdSha256SswuRo)
        },
        _ => None,
    }
}

/// The abort every structure a native does not support falls to. Carries no
/// message, which is observable in the rendered abort.
fn not_implemented() -> NativeStatus {
    NativeStatus::Abort {
        code: MOVE_ABORT_CODE_NOT_IMPLEMENTED,
        message: None,
    }
}

/// Raised when an arkworks operation fails in a way the legacy VM treats as
/// impossible, such as serializing into an in-memory buffer.
fn algebra_invariant_violation<T>() -> VMResult<T> {
    Err(native_invariant_violation(
        "aptos_std::crypto_algebra native failed unexpectedly".to_string(),
    ))
}

/// Caps the elements created within a single execution at 1 MB.
const MEMORY_LIMIT_IN_BYTES: usize = 1 << 20;

/// One stored element, with the store's byte total through and including it.
///
/// Charged bytes are a prefix sum, so the running total lets a rollback restore
/// both the length and the total by truncating alone.
struct Entry {
    obj: Box<dyn Any>,
    bytes_used: usize,
}

/// Per-transaction store of algebraic elements, indexed by handle.
///
/// One index space holds every element type. `upcast` and `downcast` hand the
/// caller's own handle back, so the same slot must be readable as `Gt` and as
/// `Fq12`. `Box<dyn Any>` also keeps the memory charge equal to the legacy VM's:
/// it charges the concrete element's size, not the widest one.
//
// TODO(perf, security): elements are held in a Rust `Vec` here; they should
// eventually live on the VM's own heap as a single rooted vector.
#[derive(Default)]
pub struct AlgebraStore {
    objs: Vec<Entry>,
    checkpoints: Vec<usize>,
}

impl AlgebraStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn bytes_used(&self) -> usize {
        self.objs.last().map_or(0, |entry| entry.bytes_used)
    }

    /// Allocates a new handle for `obj`, or aborts if the store would exceed
    /// [`MEMORY_LIMIT_IN_BYTES`].
    fn add<T: Any>(&mut self, obj: T) -> Result<u64, NativeStatus> {
        self.try_add(obj).map_err(|(used, asked)| NativeStatus::Abort {
            code: E_TOO_MUCH_MEMORY_USED,
            message: Some(format!(
                "Algebra context memory {}-byte limit exceeded: currently using {} bytes; was asked for {} bytes",
                MEMORY_LIMIT_IN_BYTES, used, asked,
            )),
        })
    }

    /// [`AlgebraStore::add`], but the abort carries no message.
    ///
    /// The legacy VM gives `rand_insecure_internal` its own message-less copy of
    /// the store macro, and whether an abort carries a message is observable.
    #[cfg(feature = "testing")]
    fn add_unmessaged<T: Any>(&mut self, obj: T) -> Result<u64, NativeStatus> {
        self.try_add(obj).map_err(|_| NativeStatus::Abort {
            code: E_TOO_MUCH_MEMORY_USED,
            message: None,
        })
    }

    /// Returns `Err((bytes_used, requested_total))` when `obj` would push the
    /// store past [`MEMORY_LIMIT_IN_BYTES`].
    fn try_add<T: Any>(&mut self, obj: T) -> Result<u64, (usize, usize)> {
        let bytes_used = self.bytes_used();
        let new_size = bytes_used + std::mem::size_of::<T>();
        if new_size > MEMORY_LIMIT_IN_BYTES {
            return Err((bytes_used, new_size));
        }
        let handle = self.objs.len() as u64;
        self.objs.push(Entry {
            obj: Box::new(obj),
            bytes_used: new_size,
        });
        Ok(handle)
    }

    /// Reads the element at `handle` as a `T`.
    ///
    /// Returns a copy: the extension borrow has to be released before the next
    /// heap allocation anyway, and every arkworks element is [`Copy`].
    fn get<T: Any + Copy>(&self, handle: u64) -> VMResult<T> {
        self.objs
            .get(handle as usize)
            .and_then(|entry| entry.obj.downcast_ref::<T>())
            .copied()
            .ok_or_else(|| {
                native_invariant_violation(format!("invalid crypto_algebra handle: {handle}"))
            })
    }
}

impl NativeExtension for AlgebraStore {
    unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {
        // Elements are referenced by handle and hold no VM heap pointers.
    }

    fn on_checkpoint(&mut self) {
        self.checkpoints.push(self.objs.len());
    }

    /// Rolls back by truncating the element vector to the checkpoint length.
    ///
    /// The watermark is the whole snapshot: an element is immutable once stored
    /// and slots are only ever appended, so a phase can only affect elements it
    /// created above the watermark. Handles never leave the store, so a
    /// rolled-back element is unobservable regardless.
    fn on_rollback(&mut self, n: usize) -> VMResult<()> {
        if n > self.checkpoints.len() {
            return Err(native_invariant_violation(format!(
                "crypto_algebra rollback({n}): only {} checkpoint(s)",
                self.checkpoints.len(),
            )));
        }
        let snapshot = self.checkpoints[self.checkpoints.len() - n];
        self.checkpoints.truncate(self.checkpoints.len() - n);
        self.objs.truncate(snapshot);
        Ok(())
    }
}

static BLS12381_GT_GENERATOR: Lazy<Option<ark_bls12_381::Fq12>> = Lazy::new(|| {
    let buf = hex::decode("b68917caaa0543a808c53908f694d1b6e7b38de90ce9d83d505ca1ef1b442d2727d7d06831d8b2a7920afc71d8eb50120f17a0ea982a88591d9f43503e94a8f1abaf2e4589f65aafb7923c484540a868883432a5c60e75860b11e5465b1c9a08873ec29e844c1c888cb396933057ffdd541b03a5220eda16b2b3a6728ea678034ce39c6839f20397202d7c5c44bb68134f93193cec215031b17399577a1de5ff1f5b0666bdd8907c61a7651e4e79e0372951505a07fa73c25788db6eb8023519a5aa97b51f1cad1d43d8aabbff4dc319c79a58cafc035218747c2f75daf8f2fb7c00c44da85b129113173d4722f5b201b6b4454062e9ea8ba78c5ca3cadaf7238b47bace5ce561804ae16b8f4b63da4645b8457a93793cbd64a7254f150781019de87ee42682940f3e70a88683d512bb2c3fb7b2434da5dedbb2d0b3fb8487c84da0d5c315bdd69c46fb05d23763f2191aabd5d5c2e12a10b8f002ff681bfd1b2ee0bf619d80d2a795eb22f2aa7b85d5ffb671a70c94809f0dafc5b73ea2fb0657bae23373b4931bc9fa321e8848ef78894e987bff150d7d671aee30b3931ac8c50e0b3b0868effc38bf48cd24b4b811a2995ac2a09122bed9fd9fa0c510a87b10290836ad06c8203397b56a78e9a0c61c77e56ccb4f1bc3d3fcaea7550f3503efe30f2d24f00891cb45620605fcfaa4292687b3a7db7c1c0554a93579e889a121fd8f72649b2402996a084d2381c5043166673b3849e4fd1e7ee4af24aa8ed443f56dfd6b68ffde4435a92cd7a4ac3bc77e1ad0cb728606cf08bf6386e5410f").ok()?;
    ark_bls12_381::Fq12::deserialize_uncompressed(buf.as_slice()).ok()
});

static BLS12381_R_LENDIAN: Lazy<Option<Vec<u8>>> = Lazy::new(|| {
    hex::decode("01000000fffffffffe5bfeff02a4bd5305d8a10908d83933487d9d2953a7ed73").ok()
});

static BLS12381_R_SCALAR: Lazy<Option<ark_ff::BigInteger256>> = Lazy::new(|| {
    BLS12381_R_LENDIAN
        .as_ref()
        .and_then(|buf| ark_ff::BigInteger256::deserialize_uncompressed(buf.as_slice()).ok())
});

static BLS12381_Q12_LENDIAN: Lazy<Option<Vec<u8>>> = Lazy::new(|| {
    hex::decode("1175f55da544c7625f8ccb1360e2b1d3ca40747811c8f5ed04440afe232b476c0215676aec05f2a44ac2da6b6d1b7cff075e7b2a587e0aab601a8d3db4f0d29906e5e4d0d78119f396d5a59f0f8d1ca8bca62540be6ab9c12d0ca00de1f311f106278d000e55a393c9766a74e0d08a298450f60d7e666575e3354bf14b8731f4e721c0c180a5ed55c2f8f51f815baecbf96b5fc717eb58ac161a27d1d5f2bdc1a079609b9d6449165b2466b32a01eac7992a1ea0cac2f223cde1d56f9bbccc67afe44621daf858df3fc0eb837818f3e42ab3e131ce4e492efa63c108e6ef91c29ed63b3045baebcb0ab8d203c7f558beaffccba31b12aca7f54b58d0c28340e4fdb3c7c94fe9c4fef9d640ff2fcff02f1748416cbed0981fbff49f0e39eaf8a30273e67ed851944d33d6a593ef5ddcd62da84568822a6045b633bf6a513b3cfe8f9de13e76f8dcbd915980dec205eab6a5c0c72dcebd9afff1d25509ddbf33f8e24131fbd74cda93336514340cf8036b66b09ed9e6a6ac37e22fb3ac407e321beae8cd9fe74c8aaeb4edaa9a7272848fc623f6fe835a2e647379f547fc5ec6371318a85bfa60009cb20ccbb8a467492988a87633c14c0324ba0d0c3e1798ed29c8494cea35023746da05e35d184b4a301d5b2238d665495c6318b5af8653758008952d06cb9e62487b196d64383c73c06d6e1cccdf9b3ce8f95679e7050d949004a55f4ccf95b2552880ae36d1f7e09504d2338316d87d14a064511a295d768113e301bdf9d4383a8be32192d3f2f3b2de14181c73839a7cb4af5301").ok()
});

static BN254_GT_GENERATOR: Lazy<Option<ark_bn254::Fq12>> = Lazy::new(|| {
    // Gt generator is defined as the `e(g1_generator, g2_generator)`.
    let buf = hex::decode("950e879d73631f5eb5788589eb5f7ef8d63e0a28de1ba00dfe4ca9ed3f252b264a8afb8eb4349db466ed1809ea4d7c39bdab7938821f1b0a00a295c72c2de002e01dbdfd0254134efcb1ec877395d25f937719b344adb1a58d129be2d6f2a9132b16a16e8ab030b130e69c69bd20b4c45986e6744a98314b5c1a0f50faa90b04dbaf9ef8aeeee3f50be31c210b598f4752f073987f9d35be8f6770d83f2ffc0af0d18dd9d2dbcdf943825acc12a7a9ddca45e629d962c6bd64908c3930a5541cfe2924dcc5580d5cef7a4bfdec90a91b59926f850d4a7923c01a5a5dbf0f5c094a2b9fb9d415820fa6b40c59bb9eade9c953407b0fc11da350a9d872cad6d3142974ca385854afdf5f583c04231adc5957c8914b6b20dc89660ed7c3bbe7c01d972be2d53ecdb27a1bcc16ac610db95aa7d237c8ff55a898cb88645a0e32530b23d7ebf5dafdd79b0f9c2ac4ba07ce18d3d16cf36e47916c4cae5d08d3afa813972c769e8514533e380c9443b3e1ee5c96fa3a0a73f301b626454721527bf900").ok()?;
    ark_bn254::Fq12::deserialize_uncompressed(buf.as_slice()).ok()
});

static BN254_R_LENDIAN: Lazy<Vec<u8>> = Lazy::new(|| BN254_R_SCALAR.to_bytes_le());
const BN254_R_SCALAR: ark_ff::BigInteger256 = ark_bn254::Fr::MODULUS;
static BN254_Q_LENDIAN: Lazy<Vec<u8>> = Lazy::new(|| BN254_Q_SCALAR.to_bytes_le());
const BN254_Q_SCALAR: ark_ff::BigInteger256 = ark_bn254::Fq::MODULUS;

/// Generated by `ark_bn254::Fq::MODULUS.pow(12)`.
static BN254_Q12_LENDIAN: Lazy<Option<Vec<u8>>> = Lazy::new(|| {
    hex::decode("21f186cad2e2d4c1dbaf8a066b0ebf41f734e3f859b1c523a6c1f4d457413fdbe3cd44add090135d3ae519acc30ee3bdb6bfac6573b767e975b18a77d53cdcddebf3672c74da9d1409d51b2b2db7ff000d59e3aa7cf09220159f925c86b65459ca6558c4eaa703bf45d85030ff85cc6a879c7e2c4034f7045faf20e4d3dcfffac5eb6634c3e7b939b69b2be70bdf6b9a4680297839b4e3a48cd746bd4d0ea82749ffb7e71bd9b3fb10aa684d71e6adab1250b1d8604d91b51c76c256a50b60ddba2f52b6cc853ac926c6ea86d09d400b2f2330e5c8e92e38905ba50a50c9e11cd979c284bf1327ccdc051a6da1a4a7eac5cec16757a27a1a2311bedd108a9b21ac0814269e7523a5dd3a1f5f4767ffe504a6cb3994fb0ec98d5cd5da00b9cb1188a85f2aa871ecb8a0f9d64141f1ccd2699c138e0ef9ac4d8d6a692b29db0f38b60eb08426ab46109fbab9a5221bb44dd338aafebcc4e6c10dd933597f3ff44ba41d04e82871447f3a759cfa9397c22c0c77f13618dfb65adc8aacf008").ok()
});

/// Production natives for the `crypto_algebra` module.
pub fn make_all_crypto_algebra_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![
        ("0x1::crypto_algebra::add_internal", arithmetic::native_add),
        ("0x1::crypto_algebra::sub_internal", arithmetic::native_sub),
        ("0x1::crypto_algebra::mul_internal", arithmetic::native_mul),
        ("0x1::crypto_algebra::div_internal", arithmetic::native_div),
        ("0x1::crypto_algebra::neg_internal", arithmetic::native_neg),
        ("0x1::crypto_algebra::inv_internal", arithmetic::native_inv),
        ("0x1::crypto_algebra::sqr_internal", arithmetic::native_sqr),
        (
            "0x1::crypto_algebra::double_internal",
            arithmetic::native_double
        ),
        ("0x1::crypto_algebra::eq_internal", arithmetic::native_eq),
        (
            "0x1::crypto_algebra::from_u64_internal",
            arithmetic::native_from_u64
        ),
        ("0x1::crypto_algebra::zero_internal", constants::native_zero),
        ("0x1::crypto_algebra::one_internal", constants::native_one),
        (
            "0x1::crypto_algebra::order_internal",
            constants::native_order
        ),
        (
            "0x1::crypto_algebra::scalar_mul_internal",
            scalar_mul::native_scalar_mul
        ),
        (
            "0x1::crypto_algebra::multi_scalar_mul_internal",
            scalar_mul::native_multi_scalar_mul
        ),
        (
            "0x1::crypto_algebra::serialize_internal",
            serialization::native_serialize
        ),
        (
            "0x1::crypto_algebra::deserialize_internal",
            serialization::native_deserialize
        ),
        (
            "0x1::crypto_algebra::pairing_internal",
            pairing::native_pairing
        ),
        (
            "0x1::crypto_algebra::multi_pairing_internal",
            pairing::native_multi_pairing
        ),
        (
            "0x1::crypto_algebra::hash_to_internal",
            hash_to::native_hash_to
        ),
        (
            "0x1::crypto_algebra::upcast_internal",
            casting::native_upcast
        ),
        (
            "0x1::crypto_algebra::downcast_internal",
            casting::native_downcast
        ),
    ]
}

/// Test-only natives for the `crypto_algebra` module.
#[cfg(feature = "testing")]
pub fn make_all_crypto_algebra_test_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    polymorphic_natives![(
        "0x1::crypto_algebra::rand_insecure_internal",
        rand::native_rand_insecure
    )]
}
