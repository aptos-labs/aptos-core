// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Point natives for the `ristretto255` module (`aptos_std::ristretto255`).
//!
//! A `RistrettoPoint` never crosses the Move/native boundary as bytes. Instead
//! each point lives in a per-transaction Rust store ([`RistrettoPointStore`]) and
//! Move holds a `u64` handle (a dense index into the store). Point natives take
//! and return handles, either as a bare `u64` or as a `&RistrettoPoint { handle:
//! u64 }` struct reference.
//!
//! The handle indirection is not about garbage collection. It matches Move
//! struct layout (`RistrettoPoint { handle: u64 }`) and keeps the decompressed
//! curve point in the store, so a chain of operations does not pay a field
//! square root to re-decompress between each step.

use crate::{
    monomorphic_natives, polymorphic_natives, ristretto255_scalar::scalar_from_bytes, NativeEntry,
};
use curve25519_dalek::{
    constants::RISTRETTO_BASEPOINT_TABLE,
    ristretto::{CompressedRistretto, RistrettoPoint},
    traits::{Identity, VartimeMultiscalarMul},
};
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeExtension,
        NativeStatus, Ref, Vector,
    },
    VMResult,
};
use sha2::Sha512;

/// A compressed ristretto point is exactly 32 bytes.
const COMPRESSED_POINT_NUM_BYTES: usize = 32;

/// Upper bound on points created within a single execution. Caps the store at
/// roughly 1.6 MB (~160 bytes per point).
const NUM_POINTS_LIMIT: usize = 10000;

/// Abort code returned when a native tries to allocate past [`NUM_POINTS_LIMIT`].
const TOO_MANY_POINTS_CREATED: u64 = aptos_types::error::resource_exhausted(4);

/// Abort code returned when an in-place point operation is given the same
/// handle for both operands.
const DUPLICATE_POINT_HANDLE: u64 = aptos_types::error::cancelled(3);

/// Per-transaction store of ristretto points, indexed by handle.
///
/// TODO(perf, security): points are held in a Rust `Vec` here; they should
/// eventually live on the VM's own heap as a single rooted vector.
#[derive(Default)]
pub struct RistrettoPointStore {
    points: Vec<RistrettoPoint>,
    checkpoints: Vec<usize>,
}

impl RistrettoPointStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocates a new handle for `point`, or aborts if too many points have
    /// been created.
    fn add(&mut self, point: RistrettoPoint) -> Result<u64, NativeStatus> {
        // Points are never deduplicated, not even identity: `*_assign` natives
        // mutate the slot at a handle in place, so sharing a slot between callers
        // would let one caller's mutation corrupt another's point.
        let id = self.points.len();
        if id >= NUM_POINTS_LIMIT {
            Err(NativeStatus::Abort {
                code: TOO_MANY_POINTS_CREATED,
                message: Some(format!(
                    "Too many points created: {}, limit is {}",
                    id, NUM_POINTS_LIMIT
                )),
            })
        } else {
            self.points.push(point);
            Ok(id as u64)
        }
    }

    /// Reads the point at `handle`.
    fn get(&self, handle: u64) -> VMResult<RistrettoPoint> {
        self.points.get(handle as usize).copied().ok_or_else(|| {
            native_invariant_violation(format!("invalid ristretto255 point handle: {handle}"))
        })
    }

    /// Updates the point at `handle`.
    fn set(&mut self, handle: u64, point: RistrettoPoint) -> VMResult<()> {
        match self.points.get_mut(handle as usize) {
            Some(slot) => {
                *slot = point;
                Ok(())
            },
            None => Err(native_invariant_violation(format!(
                "invalid ristretto255 point handle: {handle}"
            ))),
        }
    }
}

impl NativeExtension for RistrettoPointStore {
    unsafe fn relocate_roots(&mut self, _relocate: &mut dyn FnMut(*mut u8) -> Option<*mut u8>) {
        // Points are referenced by handle and hold no VM heap pointers.
    }

    fn on_checkpoint(&mut self) {
        self.checkpoints.push(self.points.len());
    }

    /// Rolls back by truncating the point vector to the checkpoint length.
    ///
    /// Correct despite in-place `set`: a `RistrettoPoint` never crosses a
    /// checkpoint boundary (it is `drop`-only and each phase is a separate
    /// top-level call), so a phase only mutates points it created above the
    /// watermark. Truncation drops exactly those; points below were never
    /// touched. Handles never leave the store, so a rolled-back point is
    /// unobservable regardless.
    fn on_rollback(&mut self, n: usize) -> VMResult<()> {
        if n > self.checkpoints.len() {
            return Err(native_invariant_violation(format!(
                "ristretto255 point rollback({n}): only {} checkpoint(s)",
                self.checkpoints.len(),
            )));
        }
        let snapshot = self.checkpoints[self.checkpoints.len() - n];
        self.checkpoints.truncate(self.checkpoints.len() - n);
        self.points.truncate(snapshot);
        Ok(())
    }
}

/// Reads the `handle` field of a `&RistrettoPoint { handle: u64 }` argument.
///
/// `RistrettoPoint` is a single inline `u64`, so the reference points straight
/// at that `u64`. Nothing needs GC rooting here: the field holds no VM heap
/// pointer and the handle is read immediately.
fn point_handle(point: &Ref<u64>) -> u64 {
    // SAFETY: `handle` is the 0th (and only) field of `RistrettoPoint`, so the
    // reference points at that `u64`.
    unsafe { core::ptr::read_unaligned(point.ptr() as *const u64) }
}

/// `0x1::ristretto255::point_identity_internal(): u64`
///
/// TODO(metering): charge gas.
pub fn native_point_identity<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let handle = match store.add(RistrettoPoint::identity()) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::point_is_canonical_internal(bytes: vector<u8>): bool`
///
/// TODO(metering): charge gas.
pub fn native_point_is_canonical<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let bytes: Vector<u8> = unsafe { ctx.arg(0)? };

    // SAFETY: byte slice consumed before any allocation. A wrong length is not
    // canonical, so it yields `false` rather than aborting.
    let is_canonical =
        match <[u8; COMPRESSED_POINT_NUM_BYTES]>::try_from(unsafe { bytes.as_bytes() }) {
            Ok(slice) => CompressedRistretto(slice).decompress().is_some(),
            Err(_) => false,
        };

    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, is_canonical)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::point_decompress_internal(bytes: vector<u8>): (u64, bool)`
///
/// TODO(metering): charge gas.
pub fn native_point_decompress<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let bytes: Vector<u8> = unsafe { ctx.arg(0)? };

    // SAFETY: byte slice consumed before any allocation.
    let point = match <[u8; COMPRESSED_POINT_NUM_BYTES]>::try_from(unsafe { bytes.as_bytes() }) {
        Ok(slice) => CompressedRistretto(slice).decompress(),
        Err(_) => None,
    };

    let Some(point) = point else {
        // An invalid encoding yields `(u64::MAX, false)`.
        // SAFETY: return slots 0 and 1 are `u64` and `bool`.
        unsafe {
            ctx.set_return(0, u64::MAX)?;
            ctx.set_return(1, false)?;
        }
        return Ok(NativeStatus::Success);
    };

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let handle = match store.add(point) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slots 0 and 1 are `u64` and `bool`.
    unsafe {
        ctx.set_return(0, handle)?;
        ctx.set_return(1, true)?;
    }
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::point_clone_internal(point_handle: u64): u64`
///
/// TODO(metering): charge gas.
pub fn native_point_clone<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `u64`.
    let handle_in = unsafe { ctx.arg::<u64>(0)? };

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let point = store.get(handle_in)?;
    let handle = match store.add(point) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::point_compress_internal(point: &RistrettoPoint): vector<u8>`
///
/// TODO(metering): charge gas.
pub fn native_point_compress<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `&RistrettoPoint`.
    let point = unsafe { ctx.arg::<Ref<u64>>(0)? };
    let handle = point_handle(&point);

    let bytes = {
        let store = ctx.get_extension::<RistrettoPointStore>()?;
        store.get(handle)?.compress().to_bytes()
    };
    let out = ctx.new_byte_vector(&bytes)?;

    // SAFETY: return slot 0 is `vector<u8>`.
    unsafe { ctx.set_return(0, out)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::point_mul_internal(point: &RistrettoPoint, a: vector<u8>, in_place: bool): u64`
///
/// TODO(metering): charge gas.
pub fn native_point_mul<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `&RistrettoPoint`, arg 1 is `vector<u8>`, arg 2 is `bool`.
    let point = unsafe { ctx.arg::<Ref<u64>>(0)? };
    let scalar_vec: Vector<u8> = unsafe { ctx.arg(1)? };
    let in_place = unsafe { ctx.arg::<bool>(2)? };
    let handle = point_handle(&point);
    // SAFETY: byte slice consumed into an owned scalar before any allocation.
    let scalar = scalar_from_bytes(unsafe { scalar_vec.as_bytes() })?;

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let result = store.get(handle)? * scalar;
    let result_handle = if in_place {
        store.set(handle, result)?;
        handle
    } else {
        match store.add(result) {
            Ok(handle) => handle,
            Err(abort) => return Ok(abort),
        }
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, result_handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::point_equals(a: &RistrettoPoint, b: &RistrettoPoint): bool`
///
/// TODO(metering): charge gas.
pub fn native_point_equals<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `&RistrettoPoint`.
    let a = unsafe { ctx.arg::<Ref<u64>>(0)? };
    let b = unsafe { ctx.arg::<Ref<u64>>(1)? };
    let a_handle = point_handle(&a);
    let b_handle = point_handle(&b);

    let equals = {
        let store = ctx.get_extension::<RistrettoPointStore>()?;
        store.get(a_handle)? == store.get(b_handle)?
    };

    // SAFETY: return slot 0 is `bool`.
    unsafe { ctx.set_return(0, equals)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::point_neg_internal(a: &RistrettoPoint, in_place: bool): u64`
///
/// TODO(metering): charge gas.
pub fn native_point_neg<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `&RistrettoPoint`, arg 1 is `bool`.
    let point = unsafe { ctx.arg::<Ref<u64>>(0)? };
    let in_place = unsafe { ctx.arg::<bool>(1)? };
    let handle = point_handle(&point);

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let result = -store.get(handle)?;
    let result_handle = if in_place {
        store.set(handle, result)?;
        handle
    } else {
        match store.add(result) {
            Ok(handle) => handle,
            Err(abort) => return Ok(abort),
        }
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, result_handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::point_add_internal(a: &RistrettoPoint, b: &RistrettoPoint, in_place: bool): u64`
///
/// TODO(metering): charge gas.
pub fn native_point_add<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `&RistrettoPoint`, arg 2 is `bool`.
    let a = unsafe { ctx.arg::<Ref<u64>>(0)? };
    let b = unsafe { ctx.arg::<Ref<u64>>(1)? };
    let in_place = unsafe { ctx.arg::<bool>(2)? };
    let a_handle = point_handle(&a);
    let b_handle = point_handle(&b);

    // In-place add requires two distinct points; aliasing both operands aborts.
    if in_place && a_handle == b_handle {
        return Ok(NativeStatus::Abort {
            code: DUPLICATE_POINT_HANDLE,
            message: Some("Duplicate Ristretto point handle".to_string()),
        });
    }

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let result = store.get(a_handle)? + store.get(b_handle)?;
    let result_handle = if in_place {
        store.set(a_handle, result)?;
        a_handle
    } else {
        match store.add(result) {
            Ok(handle) => handle,
            Err(abort) => return Ok(abort),
        }
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, result_handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::point_sub_internal(a: &RistrettoPoint, b: &RistrettoPoint, in_place: bool): u64`
///
/// TODO(metering): charge gas.
pub fn native_point_sub<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0 and 1 are `&RistrettoPoint`, arg 2 is `bool`.
    let a = unsafe { ctx.arg::<Ref<u64>>(0)? };
    let b = unsafe { ctx.arg::<Ref<u64>>(1)? };
    let in_place = unsafe { ctx.arg::<bool>(2)? };
    let a_handle = point_handle(&a);
    let b_handle = point_handle(&b);

    // In-place sub requires two distinct points; aliasing both operands aborts.
    if in_place && a_handle == b_handle {
        return Ok(NativeStatus::Abort {
            code: DUPLICATE_POINT_HANDLE,
            message: Some("Duplicate Ristretto point handle".to_string()),
        });
    }

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let result = store.get(a_handle)? - store.get(b_handle)?;
    let result_handle = if in_place {
        store.set(a_handle, result)?;
        a_handle
    } else {
        match store.add(result) {
            Ok(handle) => handle,
            Err(abort) => return Ok(abort),
        }
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, result_handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::basepoint_mul_internal(a: vector<u8>): u64`
///
/// TODO(metering): charge gas.
pub fn native_basepoint_mul<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let scalar_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    // SAFETY: byte slice consumed into an owned scalar before any allocation.
    let scalar = scalar_from_bytes(unsafe { scalar_vec.as_bytes() })?;

    let result = &RISTRETTO_BASEPOINT_TABLE * &scalar;
    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let handle = match store.add(result) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::basepoint_double_mul_internal(a: vector<u8>, some_point: &RistrettoPoint, b: vector<u8>): u64`
///
/// TODO(metering): charge gas.
pub fn native_basepoint_double_mul<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`, arg 1 is `&RistrettoPoint`, arg 2 is `vector<u8>`.
    let a_vec: Vector<u8> = unsafe { ctx.arg(0)? };
    let point = unsafe { ctx.arg::<Ref<u64>>(1)? };
    let b_vec: Vector<u8> = unsafe { ctx.arg(2)? };
    let handle = point_handle(&point);
    // SAFETY: byte slices consumed into owned scalars before any allocation.
    let a = scalar_from_bytes(unsafe { a_vec.as_bytes() })?;
    let b = scalar_from_bytes(unsafe { b_vec.as_bytes() })?;

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let point = store.get(handle)?;
    let result = RistrettoPoint::vartime_double_scalar_mul_basepoint(&a, &point, &b);
    let result_handle = match store.add(result) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, result_handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::new_point_from_sha512_internal(sha2_512_input: vector<u8>): u64`
///
/// TODO(metering): charge gas.
pub fn native_new_point_from_sha512<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let input: Vector<u8> = unsafe { ctx.arg(0)? };
    // SAFETY: byte slice consumed into an owned point before any allocation.
    let point = RistrettoPoint::hash_from_bytes::<Sha512>(unsafe { input.as_bytes() });

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let handle = match store.add(point) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::new_point_from_64_uniform_bytes_internal(bytes: vector<u8>): u64`
///
/// TODO(metering): charge gas.
pub fn native_new_point_from_64_uniform_bytes<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `vector<u8>`.
    let bytes: Vector<u8> = unsafe { ctx.arg(0)? };
    // SAFETY: byte slice copied into an owned array before any allocation. The
    // Move caller guarantees 64 bytes, so a mismatch is an invariant violation.
    let slice = <[u8; 64]>::try_from(unsafe { bytes.as_bytes() })
        .map_err(|_| native_invariant_violation("expected 64 bytes".into()))?;
    let point = RistrettoPoint::from_uniform_bytes(&slice);

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let handle = match store.add(point) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::double_scalar_mul_internal(point1: u64, point2: u64, scalar1: vector<u8>, scalar2: vector<u8>): u64`
///
/// TODO(metering): charge gas.
pub fn native_double_scalar_mul<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: args 0, 1 are `u64`; args 2, 3 are `vector<u8>`.
    let handle1 = unsafe { ctx.arg::<u64>(0)? };
    let handle2 = unsafe { ctx.arg::<u64>(1)? };
    let scalar1_vec: Vector<u8> = unsafe { ctx.arg(2)? };
    let scalar2_vec: Vector<u8> = unsafe { ctx.arg(3)? };
    // SAFETY: byte slices consumed into owned scalars before any allocation.
    let scalar1 = scalar_from_bytes(unsafe { scalar1_vec.as_bytes() })?;
    let scalar2 = scalar_from_bytes(unsafe { scalar2_vec.as_bytes() })?;

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let point1 = store.get(handle1)?;
    let point2 = store.get(handle2)?;
    let result = RistrettoPoint::vartime_multiscalar_mul([scalar1, scalar2], [point1, point2]);
    let result_handle = match store.add(result) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, result_handle)? };
    Ok(NativeStatus::Success)
}

/// `0x1::ristretto255::multi_scalar_mul_internal<P, S>(points: &vector<P>, scalars: &vector<S>): u64`
///
/// Computes `sum_i scalars[i] * points[i]`. The `<P, S>` generics are a Move
/// borrow-checker workaround; the implementation ignores the type arguments and
/// reads the arguments at their fixed runtime layout (`vector<RistrettoPoint>`
/// as inline `u64` handles, `vector<Scalar>` as `vector<vector<u8>>`). The
/// non-empty and equal-length preconditions are enforced by the Move caller.
///
/// TODO(metering): charge gas.
pub fn native_multi_scalar_mul<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `&vector<RistrettoPoint>` (inline `u64` handles); arg 1
    // is `&vector<Scalar>` (layout-identical to `vector<vector<u8>>`).
    let points_ref = unsafe { ctx.arg::<Ref<Vector<u64>>>(0)? };
    let scalars_ref = unsafe { ctx.arg::<Ref<Vector<Vector<u8>>>>(1)? };
    let points_vec = points_ref.borrow();
    let scalars_vec = scalars_ref.borrow();

    let num = scalars_vec.len();
    // Parse scalars into owned values before touching the store.
    let mut scalars = Vec::with_capacity(num as usize);
    for i in 0..num {
        let data = scalars_vec.get_element(i)?;
        // SAFETY: byte slice consumed into an owned scalar before the next read.
        scalars.push(scalar_from_bytes(unsafe { data.as_bytes() })?);
    }

    let mut store = ctx.get_extension::<RistrettoPointStore>()?;
    let mut points = Vec::with_capacity(num as usize);
    for i in 0..num {
        points.push(store.get(points_vec.get_element(i)?)?);
    }
    let result = RistrettoPoint::vartime_multiscalar_mul(scalars.iter(), points);
    let handle = match store.add(result) {
        Ok(handle) => handle,
        Err(abort) => return Ok(abort),
    };
    drop(store);

    // SAFETY: return slot 0 is `u64`.
    unsafe { ctx.set_return(0, handle)? };
    Ok(NativeStatus::Success)
}

/// Production point natives for the `ristretto255` module.
pub fn make_all_ristretto255_point_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    let mut natives = monomorphic_natives![
        (
            "0x1::ristretto255::point_identity_internal",
            native_point_identity
        ),
        (
            "0x1::ristretto255::point_is_canonical_internal",
            native_point_is_canonical
        ),
        (
            "0x1::ristretto255::point_decompress_internal",
            native_point_decompress
        ),
        (
            "0x1::ristretto255::point_clone_internal",
            native_point_clone
        ),
        (
            "0x1::ristretto255::point_compress_internal",
            native_point_compress
        ),
        ("0x1::ristretto255::point_mul_internal", native_point_mul),
        ("0x1::ristretto255::point_equals", native_point_equals),
        ("0x1::ristretto255::point_neg_internal", native_point_neg),
        ("0x1::ristretto255::point_add_internal", native_point_add),
        ("0x1::ristretto255::point_sub_internal", native_point_sub),
        (
            "0x1::ristretto255::basepoint_mul_internal",
            native_basepoint_mul
        ),
        (
            "0x1::ristretto255::basepoint_double_mul_internal",
            native_basepoint_double_mul
        ),
        (
            "0x1::ristretto255::new_point_from_sha512_internal",
            native_new_point_from_sha512
        ),
        (
            "0x1::ristretto255::new_point_from_64_uniform_bytes_internal",
            native_new_point_from_64_uniform_bytes
        ),
        (
            "0x1::ristretto255::double_scalar_mul_internal",
            native_double_scalar_mul
        ),
    ];
    // `multi_scalar_mul_internal` is generic in Move (`<P, S>`), so it is
    // registered as a polymorphic native, even though the Move caller only ever
    // instantiates it with `<RistrettoPoint, Scalar>`.
    natives.extend(polymorphic_natives![(
        "0x1::ristretto255::multi_scalar_mul_internal",
        native_multi_scalar_mul
    )]);
    natives
}
