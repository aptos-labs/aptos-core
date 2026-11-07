// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Structural equality and ordering of two VM values, driven by their layout.
//
// Host endianness: every integer goes through `MemAddr::load_unaligned`, which
// decodes the little-endian in-memory bytes on any host, and the byte-wise
// fast paths (equality over pointer-free values, address ordering) compare the
// little-endian image itself, so the walks hold on big-endian hosts by
// construction.

use crate::error::{RuntimeError, RuntimeInvariantViolation};
use mono_move_core::{
    memory::{vec_len_or_zero, MemAddr, Obj},
    types::InternedType,
    LayoutId, LayoutKind, LayoutProvider, VMInternalError, VMResult, ENUM_DATA_OFFSET,
};
use move_core_types::int256::{I256, U256};
use std::cmp::Ordering;

/// Structural equality of two non-reference values of the given type.
///
/// # Safety
///
/// Input addresses `a` and `b` must point to fully initialized values of the
/// given type.
///
/// # Precondition
///
/// For reference values, the caller must first read the reference to obtain
/// the `base` pointer to the actual data; these walks operate on the pointee.
pub unsafe fn equals<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: MemAddr,
    b: MemAddr,
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
/// Input addresses `a` and `b` must point to fully initialized values with the
/// given layout.
///
/// # Precondition
///
/// For reference values, the caller must first read the reference to obtain
/// the `base` pointer to the actual data; these walks operate on the pointee.
pub(crate) unsafe fn equals_impl<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: MemAddr,
    b: MemAddr,
    id: LayoutId,
) -> VMResult<bool> {
    // TODO(metering): This walk recurses on struct fields and vector elements; convert it
    // to a non-recursive form to bound stack depth on deeply nested values.
    let layout = layouts.layout(id).ok_or({
        RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
    })?;

    if layout.has_no_pointers_no_padding() {
        // SAFETY: both values have `layout.size` initialized bytes and, by
        // the flag, hold no pointer slots and no padding.
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
                // SAFETY: the field offset is within the value's layout, and
                // the field holds an initialized value of `field.id`.
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
            // their data for any well-typed value (null for the empty vector).
            // The length is stored in the data pointed to.
            let vec_a = unsafe { a.load_ptr() };
            let len_a = unsafe { vec_len_or_zero(vec_a) };
            let vec_b = unsafe { b.load_ptr() };
            let len_b = unsafe { vec_len_or_zero(vec_b) };

            if len_a != len_b {
                return Ok(false);
            }
            if len_a == 0 {
                return Ok(true);
            }

            let elem_layout = layouts.layout(*elem_id).ok_or({
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
            })?;
            let elem_size = elem_layout.size;
            // SAFETY: both vectors are live heap objects: the empty (null)
            // case was excluded above.
            let (obj_a, obj_b) = unsafe { (Obj::new(vec_a), Obj::new(vec_b)) };
            if elem_layout.has_no_pointers_no_padding() {
                // SAFETY: the data regions hold `len_a` elements of `elem_size`
                // bytes each, with no pointer slots and no padding by the flag.
                return Ok(unsafe {
                    bytes_cmp(
                        obj_a.vec_data(),
                        obj_b.vec_data(),
                        len_a as usize * elem_size as usize,
                    )
                    .is_eq()
                });
            }

            for i in 0..len_a {
                // SAFETY: `i < len_a == len_b`, so the ith element lies within
                // both vectors' data regions.
                let elem_a = unsafe { obj_a.vec_elem(i, elem_size) };
                let elem_b = unsafe { obj_b.vec_elem(i, elem_size) };

                // SAFETY: element addresses hold initialized values of
                // `elem_id`.
                let eq = unsafe { equals_impl(layouts, elem_a, elem_b, *elem_id)? };
                if !eq {
                    return Ok(false);
                }
            }
            Ok(true)
        },
        LayoutKind::FrozenEnum { variants, .. } => {
            // SAFETY: well-typed enum values hold non-null heap pointers and
            // every enum object stores its tag followed by the variant body.
            let obj_a = unsafe { Obj::new(a.load_ptr()) };
            let obj_b = unsafe { Obj::new(b.load_ptr()) };
            let tag_a = unsafe { obj_a.enum_tag() };
            let tag_b = unsafe { obj_b.enum_tag() };

            // Validate both tags before the equality check. An out-of-range tag
            // is heap corruption and must fail closed even when the tags differ.
            let variant_id = variants.get(tag_a as usize).map(|v| v.id).ok_or({
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

            // SAFETY: both variant bodies live at `ENUM_DATA_OFFSET` within
            // their enum objects.
            unsafe {
                equals_impl(
                    layouts,
                    obj_a.field(ENUM_DATA_OFFSET),
                    obj_b.field(ENUM_DATA_OFFSET),
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
    a: MemAddr,
    b: MemAddr,
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
/// Input addresses `a` and `b` must point to fully initialized values with the
/// given layout.
///
/// # Precondition
///
/// For reference values, the caller must first read the reference to obtain
/// the `base` pointer to the actual data; these walks operate on the pointee.
pub(crate) unsafe fn compare_impl<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: MemAddr,
    b: MemAddr,
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
            // Load the little-endian bytes as the native integer of the
            // matching width and compare numerically. `load_unaligned`
            // decodes little-endian on any host, and takes no alignment
            // requirement: these values may sit at any offset in a struct.
            //
            // SAFETY: both values hold `layout.size` initialized bytes with
            // no pointer slots.
            Ok(unsafe {
                match layout.size {
                    1 => a.load::<u8>().cmp(&b.load::<u8>()),
                    2 => a.load_unaligned::<u16>().cmp(&b.load_unaligned::<u16>()),
                    4 => a.load_unaligned::<u32>().cmp(&b.load_unaligned::<u32>()),
                    8 => a.load_unaligned::<u64>().cmp(&b.load_unaligned::<u64>()),
                    16 => a.load_unaligned::<u128>().cmp(&b.load_unaligned::<u128>()),
                    32 => a.load_unaligned::<U256>().cmp(&b.load_unaligned::<U256>()),
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
            // SAFETY: both values hold `layout.size` initialized bytes with
            // no pointer slots.
            Ok(unsafe {
                match layout.size {
                    1 => a.load::<i8>().cmp(&b.load::<i8>()),
                    2 => a.load_unaligned::<i16>().cmp(&b.load_unaligned::<i16>()),
                    4 => a.load_unaligned::<i32>().cmp(&b.load_unaligned::<i32>()),
                    8 => a.load_unaligned::<i64>().cmp(&b.load_unaligned::<i64>()),
                    16 => a.load_unaligned::<i128>().cmp(&b.load_unaligned::<i128>()),
                    32 => a.load_unaligned::<I256>().cmp(&b.load_unaligned::<I256>()),
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
            // SAFETY: both values are initialized byte arrays of `layout.size`
            // bytes with no pointer slots, as guaranteed by the precondition of
            // this function.
            Ok(unsafe { bytes_cmp(a, b, layout.size as usize) })
        },
        LayoutKind::Struct { fields } => {
            for field in fields.iter() {
                // SAFETY: the field offset is within the value's layout, and
                // the field holds an initialized value of `field.id`.
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
            // their data for any well-typed value (null for the empty vector).
            // The length is stored in the data pointed to.
            let vec_a = unsafe { a.load_ptr() };
            let len_a = unsafe { vec_len_or_zero(vec_a) };
            let vec_b = unsafe { b.load_ptr() };
            let len_b = unsafe { vec_len_or_zero(vec_b) };

            let elem = layouts.layout(*elem_id).ok_or({
                RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
            })?;
            let elem_size = elem.size;
            let common_len = len_a.min(len_b);
            if common_len == 0 {
                return Ok(len_a.cmp(&len_b));
            }
            // SAFETY: both lengths are non-zero, so both vectors are live
            // (non-null) heap objects.
            let (obj_a, obj_b) = unsafe { (Obj::new(vec_a), Obj::new(vec_b)) };
            for i in 0..common_len {
                // SAFETY: `i` is below both lengths, so the ith element lies
                // within both data regions.
                let elem_a = unsafe { obj_a.vec_elem(i, elem_size) };
                let elem_b = unsafe { obj_b.vec_elem(i, elem_size) };

                // SAFETY: element addresses hold initialized values of
                // `elem_id`.
                let ord = unsafe { compare_impl(layouts, elem_a, elem_b, *elem_id)? };
                if ord.is_ne() {
                    return Ok(ord);
                }
            }
            Ok(len_a.cmp(&len_b))
        },
        LayoutKind::FrozenEnum { variants, .. } => {
            // SAFETY: well-typed enum values hold non-null heap pointers and
            // every enum object stores its tag followed by the variant body.
            let obj_a = unsafe { Obj::new(a.load_ptr()) };
            let obj_b = unsafe { Obj::new(b.load_ptr()) };
            let tag_a = unsafe { obj_a.enum_tag() };
            let tag_b = unsafe { obj_b.enum_tag() };

            // Validate both tags before ordering them. An out-of-range tag is
            // heap corruption and must fail closed even when the tags differ.
            let variant_id = variants.get(tag_a as usize).map(|v| v.id).ok_or({
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

            // SAFETY: both variant bodies live at `ENUM_DATA_OFFSET` within
            // their enum objects.
            unsafe {
                compare_impl(
                    layouts,
                    obj_a.field(ENUM_DATA_OFFSET),
                    obj_b.field(ENUM_DATA_OFFSET),
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

/// Byte comparison of two `n`-byte regions.
///
/// # Safety
///
/// Both `a` and `b` must be readable and initialized for `n` bytes, and the
/// ranges must hold no pointer slots (they are the pointer-free bytes of a
/// value whose layout has no pointers, or an address).
unsafe fn bytes_cmp(a: MemAddr, b: MemAddr, n: usize) -> Ordering {
    // SAFETY: the caller guarantees `n` initialized, pointer-free bytes at
    // each address, and the data is not mutated for the duration of the
    // comparison.
    unsafe { a.as_bytes(n).cmp(b.as_bytes(n)) }
}
