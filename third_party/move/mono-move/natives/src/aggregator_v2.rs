// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for `0x1::aggregator_v2`. Support `u64` and `u128` integer types.
//!
//! MonoMove currently does not support parallelisation via delayed fields and
//! all aggregator operations are sequential.
//!
//! TODO(completeness): delayed fields integration needed.
//! TODO(metering): missing gas charging for natives.
//! TODO(completeness): support DerivedStringSnapshot.

use crate::{monomorphic_natives, NativeEntry};
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref,
        RootPool, VMValue,
    },
    types::{U128_TY, U64_TY},
    VMResult,
};
use std::{fmt::Display, marker::PhantomData};

/// A reference to an aggregator or a snapshot. Wraps a Move reference (a 16-byte
/// fat pointer), rooted for the lifetime of the native call so it survives GC.
struct AggregatorOrSnapshotRef<'a, T> {
    inner: Ref<'a, Opaque>,
    _t: PhantomData<T>,
}

impl<'a, T> VMValue<'a> for AggregatorOrSnapshotRef<'a, T> {
    // A reference is a 16-byte fat pointer.
    const FRAME_SLOT_SIZE: usize = 16;

    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        Self {
            inner: unsafe { Ref::read_from_frame(pool, frame_ptr, offset) },
            _t: PhantomData,
        }
    }

    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe { self.inner.write_to_frame(frame_ptr, offset) }
    }
}

impl<T: UnsignedInt> AggregatorOrSnapshotRef<'_, T> {
    fn read_value(&self) -> T {
        // SAFETY: `value` is the 0th field for **both** Aggregator and
        // AggregatorSnapshot, so we read directly from the referent without
        // extra offsets. The referent stays rooted for the call.
        unsafe { T::read(self.inner.ptr()) }
    }

    fn write_value(&self, value: T) {
        // SAFETY: `value` is the 0th field for **both** Aggregator and
        // AggregatorSnapshot, so we write directly to the referent without
        // extra offsets. The referent stays rooted for the call.
        unsafe { value.write(self.inner.ptr()) }
    }

    fn read_max_value(&self) -> T {
        // SAFETY: `max_value` is the 1st Aggregator field, so we have to add
        // the byte size of the `value` element.
        unsafe { T::read(self.inner.ptr().add(T::SIZE)) }
    }
}

/// Mirrors `0x1::aggregator_v2::Aggregator<T>` in Move.
struct Aggregator<T> {
    value: T,
    max_value: T,
}

impl<'a, T: VMValue<'a>> VMValue<'a> for Aggregator<T> {
    const FRAME_SLOT_SIZE: usize = 2 * <T as VMValue<'a>>::FRAME_SLOT_SIZE;

    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        let value = unsafe { T::read_from_frame(pool, frame_ptr, offset) };
        let max_value = unsafe {
            T::read_from_frame(
                pool,
                frame_ptr,
                offset + <T as VMValue<'a>>::FRAME_SLOT_SIZE,
            )
        };
        Self { value, max_value }
    }

    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe {
            self.value.write_to_frame(frame_ptr, offset);
            self.max_value
                .write_to_frame(frame_ptr, offset + <T as VMValue<'a>>::FRAME_SLOT_SIZE);
        }
    }
}

/// Mirrors `0x1::aggregator_v2::AggregatorSnapshot<T>` in Move.
struct AggregatorSnapshot<T> {
    value: T,
}

impl<'a, T: VMValue<'a>> VMValue<'a> for AggregatorSnapshot<T> {
    const FRAME_SLOT_SIZE: usize = <T as VMValue<'a>>::FRAME_SLOT_SIZE;

    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        // SAFETY: `value` is the 0th (and only) snapshot field.
        let value = unsafe { T::read_from_frame(pool, frame_ptr, offset) };
        Self { value }
    }

    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        // SAFETY: `value` is the 0th (and only) snapshot field.
        unsafe { self.value.write_to_frame(frame_ptr, offset) };
    }
}

/// The integer element types an aggregator holds.
trait UnsignedInt: Copy + Ord + Display + for<'a> VMValue<'a> {
    const ZERO: Self;
    const MAX: Self;
    /// Byte size of the value in a frame slot.
    const SIZE: usize;
    fn checked_add(self, other: Self) -> Option<Self>;
    fn checked_sub(self, other: Self) -> Option<Self>;

    /// Reads the value from a raw referent pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must point to [`Self::SIZE`] valid, initialized bytes.
    unsafe fn read(ptr: *const u8) -> Self;

    /// Writes the value to a raw referent pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must point to [`Self::SIZE`] writable bytes.
    unsafe fn write(self, ptr: *mut u8);
}

impl UnsignedInt for u64 {
    const MAX: Self = u64::MAX;
    const SIZE: usize = 8;
    const ZERO: Self = 0;

    fn checked_add(self, other: Self) -> Option<Self> {
        u64::checked_add(self, other)
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        u64::checked_sub(self, other)
    }

    unsafe fn read(ptr: *const u8) -> Self {
        unsafe { core::ptr::read_unaligned(ptr as *const u64) }
    }

    unsafe fn write(self, ptr: *mut u8) {
        unsafe { core::ptr::write_unaligned(ptr as *mut u64, self) }
    }
}

impl UnsignedInt for u128 {
    const MAX: Self = u128::MAX;
    const SIZE: usize = 16;
    const ZERO: Self = 0;

    fn checked_add(self, other: Self) -> Option<Self> {
        u128::checked_add(self, other)
    }

    fn checked_sub(self, other: Self) -> Option<Self> {
        u128::checked_sub(self, other)
    }

    unsafe fn read(ptr: *const u8) -> Self {
        unsafe { core::ptr::read_unaligned(ptr as *const u128) }
    }

    unsafe fn write(self, ptr: *mut u8) {
        unsafe { core::ptr::write_unaligned(ptr as *mut u128, self) }
    }
}

/// `0x1::agregator_v2::create_aggregator<T>(max_value: T): Aggregator<T>`
///
/// Creates an aggregator with the given `max_value` and zero `value`.
fn native_create_aggregator<C: NativeContext, T: UnsignedInt>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `max_value` of type `T`.
    let max_value = unsafe { ctx.arg::<T>(0) }?;
    // SAFETY: return 0 is an aggregator with layout matching VM value layout.
    unsafe {
        ctx.set_return(0, Aggregator::<T> {
            value: T::ZERO,
            max_value,
        })
    }?;
    Ok(NativeStatus::Success)
}

/// `0x1::agregator_v2::create_unbounded_aggregator<T>(): Aggregator<T>`
///
/// Creates an aggregator with bound inferred from the passed integer type
/// (its maximum) and zero `value`.
fn native_create_unbounded_aggregator<C: NativeContext, T: UnsignedInt>(
    ctx: &C,
) -> VMResult<NativeStatus> {
    // SAFETY: return 0 is an aggregator with layout matching VM value layout.
    unsafe {
        ctx.set_return(0, Aggregator::<T> {
            value: T::ZERO,
            max_value: T::MAX,
        })
    }?;
    Ok(NativeStatus::Success)
}

/// `0x1::agregator_v2::try_add<T>(self: &mut Aggregator<T>, value: T): bool`
///
/// Adds `value` to aggregators's `value` and returns true if the result is
/// at most aggregator's `max_value`. Otherwise, no-op and returns false.
fn native_try_add<C: NativeContext, T: UnsignedInt>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is aggregator reference (fat pointer); arg 1 is `T`.
    let aggregator = unsafe { ctx.arg::<AggregatorOrSnapshotRef<T>>(0) }?;
    let value = unsafe { ctx.arg::<T>(1) }?;

    let success = match aggregator.read_value().checked_add(value) {
        Some(result) if result <= aggregator.read_max_value() => {
            aggregator.write_value(result);
            true
        },
        // Overflow otherwise.
        _ => false,
    };

    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, success) }?;
    Ok(NativeStatus::Success)
}

/// `0x1::agregator_v2::try_sub<T>(self: &mut Aggregator<T>, value: T): bool`
///
/// Subtracts `value` from aggregators's `value` and returns true if the result
/// is non-negative. Otherwise, no-op and returns false indicating underflow.
fn native_try_sub<C: NativeContext, T: UnsignedInt>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is aggregator reference (fat pointer); arg 1 is `T`.
    let aggregator = unsafe { ctx.arg::<AggregatorOrSnapshotRef<T>>(0) }?;
    let rhs = unsafe { ctx.arg::<T>(1) }?;

    let success = match aggregator.read_value().checked_sub(rhs) {
        Some(result) => {
            aggregator.write_value(result);
            true
        },
        None => false,
    };

    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, success) }?;
    Ok(NativeStatus::Success)
}

/// `0x1::agregator_v2::is_at_least_impl<T>(self: &Aggregator<T>, min_amount: T): bool`
///
/// Returns true when the `value` of aggregator is at least this specified
/// minimum amount.
fn native_is_at_least_impl<C: NativeContext, T: UnsignedInt>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is aggregator reference (fat pointer), and arg 1 is the
    // amount to compare against.
    let aggregator = unsafe { ctx.arg::<AggregatorOrSnapshotRef<T>>(0) }?;
    let min_amount = unsafe { ctx.arg::<T>(1) }?;

    let result = aggregator.read_value() >= min_amount;

    // SAFETY: return 0 is `bool`.
    unsafe { ctx.set_return(0, result) }?;
    Ok(NativeStatus::Success)
}

/// `0x1::agregator_v2::read<T>(self: &Aggregator<T>): T`
///
/// Returns `value` of the aggregator.
fn native_read<C: NativeContext, T: UnsignedInt>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is aggregator reference (fat pointer).
    let aggregator = unsafe { ctx.arg::<AggregatorOrSnapshotRef<T>>(0) }?;

    let value = aggregator.read_value();
    let max_value = aggregator.read_max_value();
    if value > max_value {
        return Err(native_invariant_violation(format!(
            "Aggregator read returned value greater than max: {value} > {max_value}"
        )));
    }

    // SAFETY: return 0 is the read value.
    unsafe { ctx.set_return(0, value) }?;
    Ok(NativeStatus::Success)
}

/// `0x1::agregator_v2::snapshot<T>(self: &Aggregator<T>): AggregatorSnapshot<T>`
///
/// Captures the aggregator's current `value` into a snapshot.
fn native_snapshot<C: NativeContext, T: UnsignedInt>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is aggregator reference (fat pointer).
    let aggregator = unsafe { ctx.arg::<AggregatorOrSnapshotRef<T>>(0) }?;
    let value = aggregator.read_value();

    // SAFETY: return 0 is a snapshot with layout matching VM value layout.
    unsafe { ctx.set_return(0, AggregatorSnapshot::<T> { value }) }?;
    Ok(NativeStatus::Success)
}

/// `0x1::agregator_v2::create_snapshot<T>(value: T): AggregatorSnapshot<T>`
///
/// Wraps `value` into a snapshot.
fn native_create_snapshot<C: NativeContext, T: UnsignedInt>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `value` of type `T`.
    let value = unsafe { ctx.arg::<T>(0) }?;

    // SAFETY: return 0 is a snapshot with layout matching VM value layout.
    unsafe { ctx.set_return(0, AggregatorSnapshot::<T> { value }) }?;
    Ok(NativeStatus::Success)
}

/// `0x1::agregator_v2::read_snapshot<T>(self: &AggregatorSnapshot<T>): T`
///
/// Returns the value held by the snapshot.
fn native_read_snapshot<C: NativeContext, T: UnsignedInt>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is a snapshot reference (fat pointer).
    let snapshot = unsafe { ctx.arg::<AggregatorOrSnapshotRef<T>>(0) }?;
    let value = snapshot.read_value();

    // SAFETY: return 0 is the snapshot value.
    unsafe { ctx.set_return(0, value) }?;
    Ok(NativeStatus::Success)
}

// Only u64 and u128 types are supported.
pub fn make_all_aggregator_v2_natives<F: NativeContextFamily>() -> Vec<NativeEntry<F>> {
    monomorphic_natives![
        (
            "0x1::aggregator_v2::create_aggregator",
            &[U64_TY],
            native_create_aggregator::<_, u64>
        ),
        (
            "0x1::aggregator_v2::create_aggregator",
            &[U128_TY],
            native_create_aggregator::<_, u128>
        ),
        (
            "0x1::aggregator_v2::create_unbounded_aggregator",
            &[U64_TY],
            native_create_unbounded_aggregator::<_, u64>
        ),
        (
            "0x1::aggregator_v2::create_unbounded_aggregator",
            &[U128_TY],
            native_create_unbounded_aggregator::<_, u128>
        ),
        (
            "0x1::aggregator_v2::try_add",
            &[U64_TY],
            native_try_add::<_, u64>
        ),
        (
            "0x1::aggregator_v2::try_add",
            &[U128_TY],
            native_try_add::<_, u128>
        ),
        (
            "0x1::aggregator_v2::try_sub",
            &[U64_TY],
            native_try_sub::<_, u64>
        ),
        (
            "0x1::aggregator_v2::try_sub",
            &[U128_TY],
            native_try_sub::<_, u128>
        ),
        (
            "0x1::aggregator_v2::is_at_least_impl",
            &[U64_TY],
            native_is_at_least_impl::<_, u64>
        ),
        (
            "0x1::aggregator_v2::is_at_least_impl",
            &[U128_TY],
            native_is_at_least_impl::<_, u128>
        ),
        ("0x1::aggregator_v2::read", &[U64_TY], native_read::<_, u64>),
        (
            "0x1::aggregator_v2::read",
            &[U128_TY],
            native_read::<_, u128>
        ),
        (
            "0x1::aggregator_v2::snapshot",
            &[U64_TY],
            native_snapshot::<_, u64>
        ),
        (
            "0x1::aggregator_v2::snapshot",
            &[U128_TY],
            native_snapshot::<_, u128>
        ),
        (
            "0x1::aggregator_v2::create_snapshot",
            &[U64_TY],
            native_create_snapshot::<_, u64>
        ),
        (
            "0x1::aggregator_v2::create_snapshot",
            &[U128_TY],
            native_create_snapshot::<_, u128>
        ),
        (
            "0x1::aggregator_v2::read_snapshot",
            &[U64_TY],
            native_read_snapshot::<_, u64>
        ),
        (
            "0x1::aggregator_v2::read_snapshot",
            &[U128_TY],
            native_read_snapshot::<_, u128>
        ),
    ]
}
