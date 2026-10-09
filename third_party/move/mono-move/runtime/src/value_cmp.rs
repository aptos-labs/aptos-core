// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Structural equality and ordering of two VM values, driven by their layout.
//!
//! Both are visitors over [`walk`] with `N = 2`: the walk pairs up fields,
//! elements and variant bodies, and the visitors only compare what they are
//! handed, breaking out at the first difference.
//
// TODO(correctness): the comparison fast paths assume a little-endian host.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    value_walk::{lookup, walk, Event, Step, ValueVisitor},
};
use mono_move_core::{
    types::InternedType, LayoutId, LayoutKind, LayoutProvider, VMInternalError, VMResult,
    ValueLayout,
};
use move_core_types::int256::{I256, U256};
use std::cmp::Ordering;

/// Structural equality of two non-reference values of the given type.
///
/// # Safety
///
/// Input pointers `a` and `b` must point to fully initialized values of the
/// given type.
///
/// # Precondition
///
/// For reference values, the caller must first read the reference to obtain
/// the `base` pointer to the actual data; these walks operate on the pointee.
pub unsafe fn equals<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: *const u8,
    b: *const u8,
    ty: InternedType,
) -> VMResult<bool> {
    let id = layouts.layout_id(ty).ok_or({
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
    })?;
    // SAFETY: caller must enforce the safety precondition.
    unsafe { equals_impl(layouts, a, b, id) }
}

/// Implementation of structural equality of two values of the given layout.
///
/// # Safety
///
/// Input pointers `a` and `b` must point to fully initialized values with the
/// given layout.
///
/// # Precondition
///
/// For reference values, the caller must first read the reference to obtain
/// the `base` pointer to the actual data; these walks operate on the pointee.
pub(crate) unsafe fn equals_impl<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: *const u8,
    b: *const u8,
    id: LayoutId,
) -> VMResult<bool> {
    let layout = lookup(layouts, id)?;
    // SAFETY: caller must enforce the safety precondition.
    let differ = unsafe { walk(layouts, [a, b], layout, &mut Equals)? };
    Ok(differ.is_none())
}

/// Breaks at the first byte that differs; a completed walk means equal.
struct Equals;

impl ValueVisitor<2> for Equals {
    type Break = ();

    fn visit(&mut self, event: Event<'_, 2>) -> VMResult<Step<()>> {
        let same = match event {
            // SAFETY: every pointer handed out by the walk addresses a live
            // value of `layout`, so both are readable for `layout.size` bytes.
            Event::Scalar {
                layout,
                ptrs: [a, b],
            } => unsafe { bytes_cmp(a, b, layout.size as usize).is_eq() },
            Event::EnterStruct {
                layout,
                ptrs: [a, b],
                ..
            } => {
                if !layout.has_no_pointers_no_padding() {
                    return Ok(Step::Descend);
                }
                // SAFETY: a struct with no pointers and no padding is exactly
                // its `layout.size` bytes.
                unsafe { bytes_cmp(a, b, layout.size as usize).is_eq() }
            },
            Event::EnterVector {
                elem,
                lens: [len_a, len_b],
                data: [data_a, data_b],
                ..
            } => {
                if len_a != len_b {
                    return Ok(Step::Break(()));
                }
                if len_a == 0 {
                    return Ok(Step::Skip);
                }
                if !elem.has_no_pointers_no_padding() {
                    return Ok(Step::Descend);
                }
                // SAFETY: both vectors are non-empty, so `data` is non-null
                // and addresses `len * elem_size` bytes of elements.
                unsafe { bytes_cmp(data_a, data_b, len_a as usize * elem.size as usize).is_eq() }
            },
            // Equal tags select the same variant, whose body decides.
            Event::EnterEnum {
                tags: [tag_a, tag_b],
                ..
            } => {
                return Ok(if tag_a == tag_b {
                    Step::Descend
                } else {
                    Step::Break(())
                })
            },
            Event::Field { .. }
            | Event::Element { .. }
            | Event::ExitStruct { .. }
            | Event::ExitVector { .. }
            | Event::ExitEnum { .. } => return Ok(Step::Descend),
        };
        Ok(if same { Step::Skip } else { Step::Break(()) })
    }
}

/// Comparison of two values of the given type.
///
/// # Semantics
///
/// 1. Integers compare numerically.
/// 2. Addresses or signers (also represented as an address) compare
///    lexicographically over their bytes.
/// 3. Vectors compare lexicographically (over smaller prefix)
/// 4. Structs compare field-by-field.
/// 5. Enums compare by variant tag first, then field-by-field over the
///    matching variant's body.
///
/// # Safety
///
/// Input pointers `a` and `b` must point to fully initialized values with the
/// given layout.
///
/// # Precondition
///
/// For reference values, the caller must first read the reference to obtain
/// the `base` pointer to the actual data; these walks operate on the pointee.
pub unsafe fn compare<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: *const u8,
    b: *const u8,
    ty: InternedType,
) -> VMResult<Ordering> {
    let id = layouts.layout_id(ty).ok_or({
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
    })?;
    // SAFETY: caller must enforce the safety precondition.
    unsafe { compare_impl(layouts, a, b, id) }
}

/// Implementation of structural comparison of two non-reference values of the
/// given layout.
///
/// # Safety
///
/// Input pointers `a` and `b` must point to fully initialized values with the
/// given layout.
///
/// # Precondition
///
/// For reference values, the caller must first read the reference to obtain
/// the `base` pointer to the actual data; these walks operate on the pointee.
pub(crate) unsafe fn compare_impl<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: *const u8,
    b: *const u8,
    id: LayoutId,
) -> VMResult<Ordering> {
    let layout = lookup(layouts, id)?;
    // SAFETY: caller must enforce the safety precondition.
    let ord = unsafe { walk(layouts, [a, b], layout, &mut Compare)? };
    Ok(ord.unwrap_or(Ordering::Equal))
}

/// Breaks at the first ordered difference; a completed walk means equal.
struct Compare;

impl ValueVisitor<2> for Compare {
    type Break = Ordering;

    fn visit(&mut self, event: Event<'_, 2>) -> VMResult<Step<Ordering>> {
        let ord = match event {
            // SAFETY: every pointer handed out by the walk addresses a live
            // value of `layout`, so both are readable for `layout.size` bytes.
            Event::Scalar {
                layout,
                ptrs: [a, b],
            } => unsafe { scalar_cmp(layout, a, b)? },
            // The walk visits the common prefix; the lengths decide after it.
            Event::ExitVector {
                lens: [len_a, len_b],
                ..
            } => len_a.cmp(&len_b),
            Event::EnterEnum {
                tags: [tag_a, tag_b],
                ..
            } => tag_a.cmp(&tag_b),
            Event::EnterStruct { .. }
            | Event::EnterVector { .. }
            | Event::Field { .. }
            | Event::Element { .. }
            | Event::ExitStruct { .. }
            | Event::ExitEnum { .. } => return Ok(Step::Descend),
        };
        Ok(if ord.is_eq() {
            Step::Descend
        } else {
            Step::Break(ord)
        })
    }
}

/// Orders two scalars of the same layout.
///
/// # Safety
///
/// Both pointers must address `layout.size` readable, initialized bytes.
unsafe fn scalar_cmp(layout: &ValueLayout, a: *const u8, b: *const u8) -> VMResult<Ordering> {
    match &layout.kind {
        // A `bool` is a 1-byte `0`/`1` value, so it compares like a `u8`.
        LayoutKind::Bool | LayoutKind::UnsignedInt => {
            // Read the little-endian bytes into the native integer of the
            // matching width and compare numerically. `from_le_bytes` keeps
            // this correct on any host endianness.
            //
            // TODO(cleanup): These are unaligned, little-endian numeric reads, distinct
            // from the aligned native-endian helpers in `memory.rs`. Endianness
            // makes unifying the two non-trivial; revisit whether a shared set
            // of typed read helpers can serve both.
            //
            // SAFETY: both pointers point to a valid `layout.size`-byte region.
            Ok(unsafe {
                match layout.size {
                    1 => (*a).cmp(&*b),
                    2 => u16::from_le_bytes(read_array(a)).cmp(&u16::from_le_bytes(read_array(b))),
                    4 => u32::from_le_bytes(read_array(a)).cmp(&u32::from_le_bytes(read_array(b))),
                    8 => u64::from_le_bytes(read_array(a)).cmp(&u64::from_le_bytes(read_array(b))),
                    16 => {
                        u128::from_le_bytes(read_array(a)).cmp(&u128::from_le_bytes(read_array(b)))
                    },
                    32 => {
                        U256::from_le_bytes(read_array(a)).cmp(&U256::from_le_bytes(read_array(b)))
                    },
                    _ => {
                        return Err(VMInternalError::new(RuntimeError::InvariantViolation(
                            RuntimeInvariantViolation::Unreachable(
                                "Unexpected unsigned integer width".to_string(),
                            ),
                        )))
                    },
                }
            })
        },
        LayoutKind::SignedInt => {
            // SAFETY: both pointers point to a valid `layout.size`-byte region.
            Ok(unsafe {
                match layout.size {
                    1 => (*(a as *const i8)).cmp(&*(b as *const i8)),
                    2 => i16::from_le_bytes(read_array(a)).cmp(&i16::from_le_bytes(read_array(b))),
                    4 => i32::from_le_bytes(read_array(a)).cmp(&i32::from_le_bytes(read_array(b))),
                    8 => i64::from_le_bytes(read_array(a)).cmp(&i64::from_le_bytes(read_array(b))),
                    16 => {
                        i128::from_le_bytes(read_array(a)).cmp(&i128::from_le_bytes(read_array(b)))
                    },
                    32 => {
                        I256::from_le_bytes(read_array(a)).cmp(&I256::from_le_bytes(read_array(b)))
                    },
                    _ => {
                        return Err(VMInternalError::new(RuntimeError::InvariantViolation(
                            RuntimeInvariantViolation::Unreachable(
                                "Unexpected signed integer width".to_string(),
                            ),
                        )))
                    },
                }
            })
        },
        LayoutKind::Address | LayoutKind::Signer => {
            // SAFETY: values are valid byte arrays of the size specified by
            // the layout, as guaranteed by the precondition of this function.
            Ok(unsafe { bytes_cmp(a, b, layout.size as usize) })
        },
        LayoutKind::Struct { .. }
        | LayoutKind::Vector { .. }
        | LayoutKind::FrozenEnum { .. }
        | LayoutKind::Function
        | LayoutKind::Ref => Err(VMInternalError::new(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::Unreachable(
                "The walk reports only scalars as scalar events".to_string(),
            ),
        ))),
    }
}

/// Reads `N` bytes from the pointer into an array.
///
/// # Safety
///
/// Pointer must point to at least `N` readable, initialized bytes.
#[inline(always)]
unsafe fn read_array<const N: usize>(p: *const u8) -> [u8; N] {
    // SAFETY: `[u8; N]` has alignment 1, so this unaligned read is valid given
    // the caller's guarantee of `N` readable bytes at `p`.
    unsafe { (p as *const [u8; N]).read_unaligned() }
}

/// Byte comparison of two `n`-byte regions.
///
/// # Safety
///
/// Behavior is undefined if any of the following conditions are violated:
///
/// 1. Pointers are non-null.
/// 2. Pointers point to a single allocation of `n` bytes, allocated.
unsafe fn bytes_cmp(a: *const u8, b: *const u8, n: usize) -> Ordering {
    // SAFETY: Caller guarantees non-null pointers of the specified length into
    // a single allocation. The total size never overflows and the data is not
    // being mutated.
    unsafe {
        let a = std::slice::from_raw_parts(a, n);
        let b = std::slice::from_raw_parts(b, n);
        a.cmp(b)
    }
}
