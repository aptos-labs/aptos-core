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
pub(crate) use aptos_types::crypto::algebra::{
    BLS12381_GT_GENERATOR, BLS12381_Q12_LENDIAN, BLS12381_R_LENDIAN, BLS12381_R_SCALAR,
    BN254_GT_GENERATOR, BN254_Q12_LENDIAN, BN254_Q_LENDIAN, BN254_R_LENDIAN, BN254_R_SCALAR,
};
use aptos_types::error;
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeExtension,
        NativeStatus,
    },
    types::{view_name, view_type, view_type_list, InternedType, Type},
    VMResult,
};
use move_core_types::account_address::AccountAddress;
use std::{any::Any, cell::RefMut};

const MOVE_ABORT_CODE_INPUT_VECTOR_SIZES_NOT_MATCHING: u64 = error::invalid_argument(2);
const MOVE_ABORT_CODE_NOT_IMPLEMENTED: u64 = error::not_implemented(1);
const E_TOO_MUCH_MEMORY_USED: u64 = error::resource_exhausted(3);

const E_CONSTANTS_BLS12381GT_GT_GENERATOR_LOADING_FAILED: u64 = error::cancelled(1);
const E_CONSTANTS_BN254GT_GT_GENERATOR_LOADING_FAILED: u64 = error::cancelled(2);
const E_CONSTANTS_BLS12381_R_ORDER_LOADING_FAILED: u64 = error::cancelled(3);
const E_CONSTANTS_BLS12381FQ12_Q12_ORDER_LOADING_FAILED: u64 = error::cancelled(4);
const E_CONSTANTS_BN254FQ12_Q12_ORDER_LOADING_FAILED: u64 = error::cancelled(5);
const E_CASTING_BLS12381_R_SCALAR_LOADING_FAILED: u64 = error::cancelled(6);
const E_SERIALIZATION_BLS12381GT_CONST_LOADING_FAILED: u64 = error::cancelled(7);
#[cfg(feature = "testing")]
const E_RAND_BLS12381GT_GT_GENERATOR_LOADING_FAILED: u64 = error::cancelled(8);
#[cfg(feature = "testing")]
const E_RAND_BN254GT_GT_GENERATOR_LOADING_FAILED: u64 = error::cancelled(9);
// The legacy VM also has `cancelled(10)` for an unsupported MSM window size.
// `ark_msm_window_size` never returns `None` here, so the code is unreachable.
const E_SCALAR_MUL_MSM_COMPUTATION_FAILED: u64 = error::cancelled(11);
const E_HASH_TO_STRUCTURE_BLS12381G1_MAPPER_FAILED: u64 = error::cancelled(12);
const E_HASH_TO_STRUCTURE_BLS12381G1_HASH_FAILED: u64 = error::cancelled(13);
const E_HASH_TO_STRUCTURE_BLS12381G2_MAPPER_FAILED: u64 = error::cancelled(14);
const E_HASH_TO_STRUCTURE_BLS12381G2_HASH_FAILED: u64 = error::cancelled(15);
#[cfg(feature = "testing")]
const E_RAND_INSECURE_NOT_IMPLEMENTED: u64 = error::cancelled(16);

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

/// Matches any [`Structure`], and the unrecognized marker.
///
/// Natives that match on a tuple of markers accept only a handful of the
/// combinations. The rest fall to one arm, which spells the tuple out through
/// these macros instead of `_` so that adding a variant leaves every such match
/// non-exhaustive.
macro_rules! any_structure {
    () => {
        Some(
            Structure::BLS12381Fq12
                | Structure::BLS12381G1
                | Structure::BLS12381G2
                | Structure::BLS12381Gt
                | Structure::BLS12381Fr
                | Structure::BN254Fr
                | Structure::BN254Fq
                | Structure::BN254Fq12
                | Structure::BN254G1
                | Structure::BN254G2
                | Structure::BN254Gt,
        ) | None
    };
}

/// Matches any [`SerializationFormat`], and the unrecognized marker.
macro_rules! any_format {
    () => {
        Some(
            SerializationFormat::BLS12381Fq12LscLsb
                | SerializationFormat::BLS12381G1Compressed
                | SerializationFormat::BLS12381G1Uncompressed
                | SerializationFormat::BLS12381G2Compressed
                | SerializationFormat::BLS12381G2Uncompressed
                | SerializationFormat::BLS12381Gt
                | SerializationFormat::BLS12381FrLsb
                | SerializationFormat::BLS12381FrMsb
                | SerializationFormat::BN254G1Compressed
                | SerializationFormat::BN254G1Uncompressed
                | SerializationFormat::BN254G2Compressed
                | SerializationFormat::BN254G2Uncompressed
                | SerializationFormat::BN254Gt
                | SerializationFormat::BN254FrLsb
                | SerializationFormat::BN254FrMsb
                | SerializationFormat::BN254FqLsb
                | SerializationFormat::BN254FqMsb
                | SerializationFormat::BN254Fq12LscLsb,
        ) | None
    };
}

/// Matches any [`HashToStructureSuite`], and the unrecognized marker.
macro_rules! any_suite {
    () => {
        Some(
            HashToStructureSuite::Bls12381g1XmdSha256SswuRo
                | HashToStructureSuite::Bls12381g2XmdSha256SswuRo,
        ) | None
    };
}

pub(crate) use any_format;
pub(crate) use any_structure;
pub(crate) use any_suite;

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

/// Turns the result of [`AlgebraStore::add`] into a native returning `u64`.
///
/// `store` is taken by value so the extension borrow is released before the
/// return slot is written.
fn return_handle<C: NativeContext>(
    ctx: &C,
    store: RefMut<'_, AlgebraStore>,
    added: Result<u64, NativeStatus>,
) -> VMResult<NativeStatus> {
    let handle = match added {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// [`return_handle`] for a native returning `(bool, u64)`, where a stored
/// element is `(true, handle)`.
fn return_some_handle<C: NativeContext>(
    ctx: &C,
    store: RefMut<'_, AlgebraStore>,
    added: Result<u64, NativeStatus>,
) -> VMResult<NativeStatus> {
    let handle = match added {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slots 0 and 1 are `bool` and `u64`.
    unsafe {
        ctx.set_return(0, true)?;
        ctx.set_return(1, handle)?;
    }
    Ok(NativeStatus::Success)
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
