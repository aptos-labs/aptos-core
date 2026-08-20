// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Natives for `0x1::aggregator_v2`. Support `u64` and `u128` integer types.
//!
//! MonoMove currently does not support parallelisation via delayed fields and
//! all aggregator operations are sequential.
//!
//! TODO(completeness): delayed fields integration needed.
//! TODO(metering): missing gas charging for natives.

use crate::{monomorphic_natives, NativeEntry};
use aptos_types::{
    delayed_fields::{calculate_width_for_constant_string, U128_MAX_DIGITS, U64_MAX_DIGITS},
    error,
    serde_helper::bcs_utils::{bcs_size_of_byte_array, size_u32_as_uleb128},
};
use mono_move_core::{
    native::{
        native_invariant_violation, NativeContext, NativeContextFamily, NativeStatus, Opaque, Ref,
        RootPool, VMValue, Vector,
    },
    types::{U128_TY, U64_TY},
    VMResult,
};
use std::{fmt::Display, marker::PhantomData};

/// Matches the framework limit on derived-string input length.
const DERIVED_STRING_INPUT_MAX_LENGTH: usize = 1024;

/// Matches the native abort code for over-long derived-string input.
const EINPUT_STRING_LENGTH_TOO_LARGE: u64 = error::invalid_state(8);

/// A reference to an aggregator or a snapshot. Wraps a Move reference (a 16-byte
/// fat pointer), rooted for the lifetime of the native call so it survives GC.
struct AggregatorOrSnapshotRef<'a, T> {
    inner: Ref<'a, Opaque>,
    _t: PhantomData<T>,
}

impl<'a, T> VMValue<'a> for AggregatorOrSnapshotRef<'a, T> {
    // A reference is a 16-byte fat pointer.
    const FRAME_SLOT_SIZE: usize = 16;

    /// # Safety
    ///
    /// `offset..offset + FRAME_SLOT_SIZE` must lie within the current frame's
    /// arg / return region.
    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        Self {
            inner: unsafe { Ref::read_from_frame(pool, frame_ptr, offset) },
            _t: PhantomData,
        }
    }

    /// # Safety
    ///
    /// `offset..offset + FRAME_SLOT_SIZE` must lie within the current frame's
    /// arg / return region.
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

    /// # Safety
    ///
    /// `offset..offset + FRAME_SLOT_SIZE` must lie within the current frame's
    /// arg / return region.
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

    /// # Safety
    ///
    /// `offset..offset + FRAME_SLOT_SIZE` must lie within the current frame's
    /// arg / return region.
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

    /// # Safety
    ///
    /// `offset..offset + FRAME_SLOT_SIZE` must lie within the current frame's
    /// arg / return region.
    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        // SAFETY: `value` is the 0th (and only) snapshot field.
        let value = unsafe { T::read_from_frame(pool, frame_ptr, offset) };
        Self { value }
    }

    /// # Safety
    ///
    /// `offset..offset + FRAME_SLOT_SIZE` must lie within the current frame's
    /// arg / return region.
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        // SAFETY: `value` is the 0th (and only) snapshot field.
        unsafe { self.value.write_to_frame(frame_ptr, offset) };
    }
}

/// Mirrors `0x1::aggregator_v2::DerivedStringSnapshot` in Move. Both fields are
/// heap-boxed vectors, so each occupies an 8-byte pointer in the frame: the
/// single-field `String` is flattened to its `bytes` vector, and `padding` is a
/// `vector<u8>`.
struct DerivedStringSnapshot<'a> {
    value: Vector<'a, u8>,
    padding: Vector<'a, u8>,
}

impl<'a> VMValue<'a> for DerivedStringSnapshot<'a> {
    const FRAME_SLOT_SIZE: usize = 2 * <Vector<'a, u8> as VMValue<'a>>::FRAME_SLOT_SIZE;

    /// # Safety
    ///
    /// `offset..offset + FRAME_SLOT_SIZE` must lie within the current frame's
    /// arg / return region.
    unsafe fn read_from_frame(pool: &'a RootPool, frame_ptr: *const u8, offset: usize) -> Self {
        unsafe {
            let value = Vector::read_from_frame(pool, frame_ptr, offset);
            let padding = Vector::read_from_frame(
                pool,
                frame_ptr,
                offset + <Vector<'a, u8> as VMValue<'a>>::FRAME_SLOT_SIZE,
            );
            Self { value, padding }
        }
    }

    /// # Safety
    ///
    /// `offset..offset + FRAME_SLOT_SIZE` must lie within the current frame's
    /// arg / return region.
    unsafe fn write_to_frame(self, frame_ptr: *mut u8, offset: usize) {
        unsafe {
            self.value.write_to_frame(frame_ptr, offset);
            self.padding.write_to_frame(
                frame_ptr,
                offset + <Vector<'a, u8> as VMValue<'a>>::FRAME_SLOT_SIZE,
            );
        }
    }
}

/// The integer element types an aggregator holds.
trait UnsignedInt: Copy + Ord + Display + for<'a> VMValue<'a> {
    const ZERO: Self;
    const MAX: Self;
    /// Byte size of the value in a frame slot.
    const SIZE: usize;
    /// Number of decimal digits in [`Self::MAX`]; the worst-case rendered width.
    const MAX_DECIMAL_DIGITS: usize;
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
    const MAX_DECIMAL_DIGITS: usize = U64_MAX_DIGITS;
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
    const MAX_DECIMAL_DIGITS: usize = U128_MAX_DIGITS;
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

/// Builds a `DerivedStringSnapshot` whose BCS serialization is exactly `width`
/// bytes wide, padding the `value` string with trailing zero bytes. Mirrors
/// `bytes_and_width_to_derived_string_struct`, so the serialized layout matches
/// byte-for-byte.
fn build_derived_string_snapshot<C: NativeContext>(
    ctx: &C,
    bytes: Vec<u8>,
    width: usize,
) -> VMResult<NativeStatus> {
    let value_width = bcs_size_of_byte_array(bytes.len());
    // The padding vector needs at least its own 1-byte length prefix.
    let padding_len = width.checked_sub(value_width + 1).ok_or_else(|| {
        native_invariant_violation(format!(
            "DerivedStringSnapshot has no space for padding: value_width {value_width}, width {width}"
        ))
    })?;
    // Padding stays short enough to serialize its length in a single byte.
    if size_u32_as_uleb128(padding_len) > 1 {
        return Err(native_invariant_violation(format!(
            "DerivedStringSnapshot padding is too large: padding_len {padding_len}, width {width}"
        )));
    }

    let value = ctx.new_byte_vector(&bytes)?;
    let padding = ctx.new_byte_vector(&vec![0u8; padding_len])?;
    // SAFETY: return 0 is `DerivedStringSnapshot { value: String, padding: vector<u8> }`.
    unsafe { ctx.set_return(0, DerivedStringSnapshot { value, padding }) }?;
    Ok(NativeStatus::Success)
}

/// `0x1::aggregator_v2::create_derived_string(value: String): DerivedStringSnapshot`
///
/// Wraps `value` into a derived string snapshot, padded so that its serialized
/// width is fixed.
//
// TODO(metering): charge gas for the input-length-proportional work.
fn native_create_derived_string<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `value: String`, which flattens to `vector<u8>`.
    let value = unsafe { ctx.arg::<Vector<u8>>(0) }?;
    // Copy off the VM heap before any allocation may relocate it.
    let value_bytes = unsafe { value.as_bytes() }.to_vec();

    if value_bytes.len() > DERIVED_STRING_INPUT_MAX_LENGTH {
        return Ok(NativeStatus::Abort {
            code: EINPUT_STRING_LENGTH_TOO_LARGE,
            message: Some(format!(
                "Derived string snapshot input length ({}) exceeds maximum {}",
                value_bytes.len(),
                DERIVED_STRING_INPUT_MAX_LENGTH
            )),
        });
    }

    let width = calculate_width_for_constant_string(value_bytes.len());
    build_derived_string_snapshot(ctx, value_bytes, width)
}

/// `0x1::aggregator_v2::derive_string_concat<T>(before: String, snapshot: &AggregatorSnapshot<T>, after: String): DerivedStringSnapshot`
///
/// Concatenates `before`, the snapshot's decimal value, and `after` into a
/// derived string snapshot, padded so that its serialized width is fixed.
//
// TODO(metering): charge gas for the input-length-proportional work.
fn native_derive_string_concat<C: NativeContext, T: UnsignedInt>(
    ctx: &C,
) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `before: String` (flattens to `vector<u8>`), arg 1 is the
    // snapshot reference (fat pointer), and arg 2 is `after: String`.
    let before = unsafe { ctx.arg::<Vector<u8>>(0) }?;
    let snapshot = unsafe { ctx.arg::<AggregatorOrSnapshotRef<T>>(1) }?;
    let after = unsafe { ctx.arg::<Vector<u8>>(2) }?;

    // Copy the strings off the VM heap and read the snapshot value before any
    // allocation, which may relocate the borrowed bytes.
    let prefix = unsafe { before.as_bytes() }.to_vec();
    let suffix = unsafe { after.as_bytes() }.to_vec();
    let value = snapshot.read_value();

    if prefix.len() + suffix.len() > DERIVED_STRING_INPUT_MAX_LENGTH {
        return Ok(NativeStatus::Abort {
            code: EINPUT_STRING_LENGTH_TOO_LARGE,
            message: Some(format!(
                "Derived string snapshot input length ({} + {}) exceeds maximum {}",
                prefix.len(),
                suffix.len(),
                DERIVED_STRING_INPUT_MAX_LENGTH
            )),
        });
    }

    // The width reserves room for the worst-case (max-digit) integer, so the
    // actual (shorter) value is padded up to a fixed width.
    let width = bcs_size_of_byte_array(prefix.len() + suffix.len() + T::MAX_DECIMAL_DIGITS) + 1;

    // output = before ++ decimal(value) ++ after.
    let mut output = prefix;
    output.extend_from_slice(value.to_string().as_bytes());
    output.extend_from_slice(&suffix);

    build_derived_string_snapshot(ctx, output, width)
}

/// `0x1::aggregator_v2::read_derived_string(self: &DerivedStringSnapshot): String`
///
/// Returns the `value` string held by the derived string snapshot, dropping the
/// padding.
//
// TODO(metering): charge gas for the input-length-proportional copy.
fn native_read_derived_string<C: NativeContext>(ctx: &C) -> VMResult<NativeStatus> {
    // SAFETY: arg 0 is `&DerivedStringSnapshot`. Its 0th field, `value`, is a
    // flattened `String`, i.e. a `vector<u8>` at offset 0 of the referent.
    let snapshot = unsafe { ctx.arg::<Ref<Vector<u8>>>(0) }?;
    // Copy the bytes off the VM heap before allocating: `new_byte_vector` may
    // trigger a GC that relocates them.
    let bytes = {
        let value = snapshot.borrow();
        // SAFETY: the bytes are consumed immediately into an owned Vec.
        unsafe { value.as_bytes() }.to_vec()
    };

    let value = ctx.new_byte_vector(&bytes)?;
    // SAFETY: return 0 is `String`, flattened to `vector<u8>`.
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
        (
            "0x1::aggregator_v2::create_derived_string",
            native_create_derived_string
        ),
        (
            "0x1::aggregator_v2::read_derived_string",
            native_read_derived_string
        ),
        (
            "0x1::aggregator_v2::derive_string_concat",
            &[U64_TY],
            native_derive_string_concat::<_, u64>
        ),
        (
            "0x1::aggregator_v2::derive_string_concat",
            &[U128_TY],
            native_derive_string_concat::<_, u128>
        ),
    ]
}
