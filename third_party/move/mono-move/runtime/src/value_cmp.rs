// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Structural equality and ordering of two VM values, driven by their layout.
//
// TODO(correctness): the comparison fast paths assume a little-endian host.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    memory::{read_enum_tag, read_ptr, read_vec_len},
    types::VEC_DATA_OFFSET,
};
use mono_move_core::{
    types::InternedType, LayoutId, LayoutKind, LayoutProvider, VMInternalError, VMResult,
    ENUM_DATA_OFFSET,
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
    // TODO(metering): This walk recurses on struct fields and vector elements; convert it
    // to a non-recursive form to bound stack depth on deeply nested values.
    let layout = layouts.layout(id).ok_or({
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
    })?;

    if layout.has_no_pointers_no_padding() {
        // SAFETY: both pointers must have layout's size and have no pointers,
        // no padding.
        return Ok(unsafe { bytes_cmp(a, b, layout.size as usize).is_eq() });
    }

    match &layout.kind {
        LayoutKind::Bool
        | LayoutKind::UnsignedInt
        | LayoutKind::SignedInt
        | LayoutKind::Address
        | LayoutKind::Signer => Err(VMInternalError::new(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::Unreachable(
                "Primitive layouts must be handled by fast-path".to_string(),
            ),
        ))),
        LayoutKind::Struct { fields } => {
            for field in fields.iter() {
                // SAFETY: value is a valid struct, so all fields lie at `offset`
                // and are within bounds.
                let eq = unsafe {
                    equals_impl(
                        layouts,
                        a.add(field.offset as usize),
                        b.add(field.offset as usize),
                        field.id,
                    )?
                };
                if !eq {
                    return Ok(false);
                }
            }
            Ok(true)
        },
        LayoutKind::Vector { elem_id, .. } => {
            // SAFETY: vector values hold 8-byte heap pointers pointing to
            // their data for any well-typed value. The length is stored in
            // the data pointed to.
            let vec_a = unsafe { read_ptr(a, 0usize) };
            let len_a = unsafe { read_vec_len(vec_a) };
            let vec_b = unsafe { read_ptr(b, 0usize) };
            let len_b = unsafe { read_vec_len(vec_b) };

            if len_a != len_b {
                return Ok(false);
            }
            if len_a == 0 {
                return Ok(true);
            }

            let elem_layout = layouts.layout(*elem_id).ok_or({
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
            })?;
            let elem_size = elem_layout.size as usize;
            if elem_layout.has_no_pointers_no_padding() {
                // SAFETY: both vectors have same size specified by the layout.
                let data_a = unsafe { vec_a.add(VEC_DATA_OFFSET) };
                let data_b = unsafe { vec_b.add(VEC_DATA_OFFSET) };
                return Ok(unsafe {
                    bytes_cmp(data_a, data_b, len_a as usize * elem_size).is_eq()
                });
            }

            for i in 0..len_a as usize {
                // SAFETY: ith element lies within the vector data region,
                // so the pointer is non-null and new pointer points within
                // the data region. Lengths of `a` and `b` are the same.
                let elem_a = unsafe { vec_a.add(VEC_DATA_OFFSET + i * elem_size) };
                let elem_b = unsafe { vec_b.add(VEC_DATA_OFFSET + i * elem_size) };

                // SAFETY: element pointers point to valid vector element
                // values.
                let eq = unsafe { equals_impl(layouts, elem_a, elem_b, *elem_id)? };
                if !eq {
                    return Ok(false);
                }
            }
            Ok(true)
        },
        LayoutKind::FrozenEnum { variants, .. } => {
            // SAFETY: well-typed enum values hold non-null heap pointers and
            // every enum object stores its data following the offset.
            let obj_a = unsafe { read_ptr(a, 0usize) };
            let obj_b = unsafe { read_ptr(b, 0usize) };
            let tag_a = unsafe { read_enum_tag(obj_a) };
            let tag_b = unsafe { read_enum_tag(obj_b) };

            // Validate both tags before the equality check. An out-of-range tag
            // is heap corruption and must fail closed even when the tags differ.
            let variant_id = *variants.get(tag_a as usize).ok_or({
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::EnumTagOutOfRange {
                    tag: tag_a,
                    variant_count: variants.len(),
                })
            })?;
            if tag_b as usize >= variants.len() {
                return Err(VMInternalError::new(RuntimeError::InvariantViolation(
                    RuntimeInvariantViolation::EnumTagOutOfRange {
                        tag: tag_b,
                        variant_count: variants.len(),
                    },
                )));
            }

            if tag_a != tag_b {
                return Ok(false);
            }

            // SAFETY: both variant bodies live at the specified offset.
            unsafe {
                equals_impl(
                    layouts,
                    obj_a.add(ENUM_DATA_OFFSET),
                    obj_b.add(ENUM_DATA_OFFSET),
                    variant_id,
                )
            }
        },
        // TODO(completeness): function values are not yet supported.
        LayoutKind::Function => Err(VMInternalError::new(RuntimeError::Unsupported(
            "function values are not yet supported",
        ))),
        LayoutKind::Ref => Err(VMInternalError::new(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::Unreachable(
                "Equality runs on pointee types only".to_string(),
            ),
        ))),
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
    // TODO(metering): This walk recurses on struct fields and vector elements; convert it
    // to a non-recursive form to bound stack depth on deeply nested values.
    let layout = layouts.layout(id).ok_or({
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
    })?;
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
        LayoutKind::Struct { fields } => {
            for field in fields.iter() {
                // SAFETY: value is a valid struct, so all fields lie at `offset`
                // and are within bounds.
                let ord = unsafe {
                    compare_impl(
                        layouts,
                        a.add(field.offset as usize),
                        b.add(field.offset as usize),
                        field.id,
                    )?
                };
                if ord.is_ne() {
                    return Ok(ord);
                }
            }
            Ok(Ordering::Equal)
        },
        LayoutKind::Vector { elem_id, .. } => {
            // SAFETY: vector values hold 8-byte heap pointers pointing to
            // their data for any well-typed value. The length is stored in
            // the data pointed to.
            let vec_a = unsafe { read_ptr(a, 0usize) };
            let len_a = unsafe { read_vec_len(vec_a) };
            let vec_b = unsafe { read_ptr(b, 0usize) };
            let len_b = unsafe { read_vec_len(vec_b) };

            let elem = layouts.layout(*elem_id).ok_or({
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
            })?;
            let elem_size = elem.size as usize;
            for i in 0..len_a.min(len_b) as usize {
                // SAFETY: ith element lies within the vector data region,
                // so the pointer is non-null and new pointer points within
                // the data region.
                let elem_a = unsafe { vec_a.add(VEC_DATA_OFFSET + i * elem_size) };
                let elem_b = unsafe { vec_b.add(VEC_DATA_OFFSET + i * elem_size) };

                // SAFETY: element pointers point to valid values.
                let ord = unsafe { compare_impl(layouts, elem_a, elem_b, *elem_id)? };
                if ord.is_ne() {
                    return Ok(ord);
                }
            }
            Ok(len_a.cmp(&len_b))
        },
        LayoutKind::FrozenEnum { variants, .. } => {
            // SAFETY: well-typed enum values hold non-null heap pointers and
            // every enum object stores its tag followed by data payload.
            let obj_a = unsafe { read_ptr(a, 0usize) };
            let obj_b = unsafe { read_ptr(b, 0usize) };
            let tag_a = unsafe { read_enum_tag(obj_a) };
            let tag_b = unsafe { read_enum_tag(obj_b) };

            // Validate both tags before ordering them. An out-of-range tag is
            // heap corruption and must fail closed even when the tags differ.
            let variant_id = *variants.get(tag_a as usize).ok_or({
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::EnumTagOutOfRange {
                    tag: tag_a,
                    variant_count: variants.len(),
                })
            })?;
            if tag_b as usize >= variants.len() {
                return Err(VMInternalError::new(RuntimeError::InvariantViolation(
                    RuntimeInvariantViolation::EnumTagOutOfRange {
                        tag: tag_b,
                        variant_count: variants.len(),
                    },
                )));
            }

            let ord = tag_a.cmp(&tag_b);
            if ord.is_ne() {
                return Ok(ord);
            }

            // SAFETY: both variant bodies live at the specified offset.
            unsafe {
                compare_impl(
                    layouts,
                    obj_a.add(ENUM_DATA_OFFSET),
                    obj_b.add(ENUM_DATA_OFFSET),
                    variant_id,
                )
            }
        },
        // TODO(completeness): function values are not yet supported.
        LayoutKind::Function => Err(VMInternalError::new(RuntimeError::Unsupported(
            "function values are not yet supported",
        ))),
        LayoutKind::Ref => Err(VMInternalError::new(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::Unreachable(
                "Comparison runs on pointee types only".to_string(),
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
