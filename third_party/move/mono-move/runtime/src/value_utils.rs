// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Operations over value trees driven by value layouts: BCS-serializing an
//! in-memory value and reporting its serialized size (fixed when the type
//! allows, otherwise computed by walking the value), deserializing BCS bytes
//! back into the flat in-memory representation (allocating heap storage for
//! vectors, and failing rather than running GC when heap space runs out), and
//! comparing two values for structural equality and ordering.
//!
//! TODO(correctness):
//!   Current implementation works only for little-endian architectures,
//!   we should revisit it to have big-endian working as well. In particular,
//!   serialization fast-path via memcpy or integer comparison are broken for
//!   big endian hosts.
//!
//! TODO(cleanup):
//!   Unify these value walks (serialize, deserialize, equals, compare) under a
//!   shared visitor/fold abstraction instead of four parallel recursive
//!   implementations.
//!
//! TODO(testing):
//!   Add differential tests for these walks against the existing Move VM.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    heap::{heap_alloc, AllocationResult, Heap},
    memory::{read_enum_tag, read_ptr, read_vec_len, write_enum_tag, write_ptr, write_u64},
    types::{VEC_DATA_OFFSET, VEC_LENGTH_OFFSET},
};
use mono_move_core::{
    types::InternedType, LayoutId, LayoutKind, LayoutProvider, VMInternalError, VMResult,
    ValueLayout, ENUM_DATA_OFFSET, OBJECT_HEADER_SIZE,
};
use move_core_types::int256::{I256, U256};
use std::cmp::Ordering;

/// Returns the fixed BCS size of a value of the given type, or [`None`] when it
/// is data-dependent (e.g., for vectors, enums, function values, etc.).
pub fn fixed_serialized_size<T: LayoutProvider + ?Sized>(
    layouts: &T,
    ty: InternedType,
) -> VMResult<Option<usize>> {
    let layout = layouts.layout_by_ty(ty).ok_or_else(layout_not_found)?;
    Ok(layout.fixed_serialized_size().map(|n| n as usize))
}

/// Returns the BCS serialized size the value stored at `base` of the given type.
///
/// # Safety
///
/// `base` must point to a fully initialized value of the given type, and must
/// remain valid (with all reachable heap objects live) throughout the call.
#[allow(dead_code)]
pub unsafe fn serialized_size<T: LayoutProvider + ?Sized>(
    layouts: &T,
    base: *const u8,
    ty: InternedType,
) -> VMResult<usize> {
    // TODO(perf): Implement a more efficient serialized size implementation:
    //   - Use constant serialized size as fast path
    //   - Avoid allocations into buffer when serializing.
    unsafe { serialize(layouts, base, ty).map(|bytes| bytes.len()) }
}

/// BCS-serializes the value stored at `base` of the given type.
///
/// # Safety
///
/// `base` must point to a fully initialized value of the given type, and must
/// remain valid (with all reachable heap objects live) throughout the call.
#[allow(dead_code)]
pub unsafe fn serialize<T: LayoutProvider + ?Sized>(
    layouts: &T,
    base: *const u8,
    ty: InternedType,
) -> VMResult<Vec<u8>> {
    let layout = layouts.layout_by_ty(ty).ok_or_else(layout_not_found)?;

    let mut out = vec![];
    if let Some(serialized_size) = layout.fixed_serialized_size() {
        out.reserve(serialized_size as usize);
    }

    // SAFETY: precondition enforced by the caller guarantees safety.
    unsafe { serialize_impl(layouts, base, layout, &mut out)? };
    Ok(out)
}

/// Implementation of BCS serialization of a value with the given layout.
///
/// # Safety
///
/// `base` must point to a fully initialized value of the given type, and must
/// remain valid (with all reachable heap objects live) throughout the call.
unsafe fn serialize_impl<T: LayoutProvider + ?Sized>(
    layouts: &T,
    base: *const u8,
    layout: &ValueLayout,
    out: &mut Vec<u8>,
) -> VMResult<()> {
    // TODO(metering): This walk recurses on struct fields and vector elements; convert it
    // to a non-recursive form to bound stack depth on deeply nested values.
    if layout.has_no_pointers_no_padding() {
        // SAFETY: for values with no padding and pointers, value's in-memory
        // bytes are its BCS encoding.
        // TODO(correctness): breaks on big-endian hosts. The in-memory
        // representation is native-endian, so this raw copy only equals the
        // little-endian BCS encoding on little-endian hosts.
        unsafe { out.extend_from_slice(std::slice::from_raw_parts(base, layout.size as usize)) };
        return Ok(());
    }

    match &layout.kind {
        LayoutKind::Bool
        | LayoutKind::UnsignedInt
        | LayoutKind::SignedInt
        | LayoutKind::Address => Err(VMInternalError::new(unreachable(
            "Primitive types have no padding / pointers and must be already handled",
        ))),
        LayoutKind::Struct { fields } => {
            for field in fields.iter() {
                let field_layout = layouts.layout(field.id).ok_or_else(layout_not_found)?;
                // SAFETY: the field lies within `base`'s region at `offset`
                // which holds for well-typed values, as guaranteed by the
                // safety precondition of this function.
                unsafe {
                    serialize_impl(layouts, base.add(field.offset as usize), field_layout, out)?
                };
            }
            Ok(())
        },
        LayoutKind::Vector { elem_id, .. } => {
            // SAFETY: vector value holds an 8-byte heap pointer pointing to
            // its data for any well-typed value. The length is stored in the
            // data pointed to.
            let vec_ptr = unsafe { read_ptr(base, 0usize) };
            let len = unsafe { read_vec_len(vec_ptr) };
            if len > bcs::MAX_SEQUENCE_LENGTH as u64 {
                return Err(VMInternalError::new(RuntimeError::BCSSequenceTooLong {
                    len,
                }));
            }
            write_uleb128_len(out, len);
            if len == 0 {
                return Ok(());
            }

            let elem_layout = layouts.layout(*elem_id).ok_or_else(layout_not_found)?;
            let elem_size = elem_layout.size as usize;
            if elem_layout.has_no_pointers_no_padding() {
                // TODO(correctness): breaks on big-endian hosts, for the same
                // reason as the scalar fast path: native-endian in-memory bytes
                // equal the little-endian BCS bytes only on little-endian hosts.
                // SAFETY: vector data is a single allocation, pointer is not
                // null and is within bounds.
                let vec_data = unsafe {
                    std::slice::from_raw_parts(
                        vec_ptr.add(VEC_DATA_OFFSET),
                        len as usize * elem_size,
                    )
                };
                out.extend_from_slice(vec_data);
            } else {
                for i in 0..len as usize {
                    // SAFETY: ith element lies within the vector data region,
                    // so the pointer is non-null and new pointer points within
                    // the data region.
                    let elem_ptr = unsafe { vec_ptr.add(VEC_DATA_OFFSET + i * elem_size) };
                    // SAFETY: element pointer is a valid value of the given
                    // element layout, as guaranteed by the valid `base` value
                    // pointer passed into this function.
                    unsafe { serialize_impl(layouts, elem_ptr, elem_layout, out)? };
                }
            }
            Ok(())
        },
        LayoutKind::FrozenEnum { variants, .. } => {
            // SAFETY: an enum value holds a non-null heap pointer and data
            // follows the offset.
            let obj_ptr = unsafe { read_ptr(base, 0usize) };
            let tag = unsafe { read_enum_tag(obj_ptr) };
            let variant_id = variants
                .get(tag as usize)
                .ok_or_else(|| enum_tag_out_of_range(tag, variants.len()))?;
            write_uleb128_len(out, tag);

            let variant_layout = layouts.layout(*variant_id).ok_or_else(layout_not_found)?;
            // SAFETY: the variant body lives at the specified offset within
            // the object.
            unsafe { serialize_impl(layouts, obj_ptr.add(ENUM_DATA_OFFSET), variant_layout, out)? };
            Ok(())
        },
        LayoutKind::Function => {
            // TODO(completeness): function values are not yet supported.
            todo!("function values are not yet supported");
        },
        LayoutKind::Ref => Err(VMInternalError::new(unreachable(
            "References cannot be serialized",
        ))),
    }
}

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
#[allow(dead_code)]
pub unsafe fn equals<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: *const u8,
    b: *const u8,
    ty: InternedType,
) -> VMResult<bool> {
    let id = layouts.layout_id(ty).ok_or_else(layout_not_found)?;
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
unsafe fn equals_impl<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: *const u8,
    b: *const u8,
    id: LayoutId,
) -> VMResult<bool> {
    // TODO(metering): This walk recurses on struct fields and vector elements; convert it
    // to a non-recursive form to bound stack depth on deeply nested values.
    let layout = layouts.layout(id).ok_or_else(layout_not_found)?;

    if layout.has_no_pointers_no_padding() {
        // SAFETY: both pointers must have layout's size and have no pointers,
        // no padding.
        return Ok(unsafe { bytes_cmp(a, b, layout.size as usize).is_eq() });
    }

    match &layout.kind {
        LayoutKind::Bool
        | LayoutKind::UnsignedInt
        | LayoutKind::SignedInt
        | LayoutKind::Address => Err(VMInternalError::new(unreachable(
            "Primitive layouts must be handled by fast-path",
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

            let elem_layout = layouts.layout(*elem_id).ok_or_else(layout_not_found)?;
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
            let variant_id = *variants
                .get(tag_a as usize)
                .ok_or_else(|| enum_tag_out_of_range(tag_a, variants.len()))?;
            if tag_b as usize >= variants.len() {
                return Err(VMInternalError::new(enum_tag_out_of_range(
                    tag_b,
                    variants.len(),
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
        LayoutKind::Function => {
            // TODO(completeness): function values are not yet supported.
            todo!("function values are not yet supported");
        },
        LayoutKind::Ref => Err(VMInternalError::new(unreachable(
            "Equality runs on pointee types only",
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
#[allow(dead_code)]
pub unsafe fn compare<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: *const u8,
    b: *const u8,
    ty: InternedType,
) -> VMResult<Ordering> {
    let id = layouts.layout_id(ty).ok_or_else(layout_not_found)?;
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
unsafe fn compare_impl<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: *const u8,
    b: *const u8,
    id: LayoutId,
) -> VMResult<Ordering> {
    // TODO(metering): This walk recurses on struct fields and vector elements; convert it
    // to a non-recursive form to bound stack depth on deeply nested values.
    let layout = layouts.layout(id).ok_or_else(layout_not_found)?;
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
                        return Err(VMInternalError::new(unreachable(
                            "Unexpected unsigned integer width",
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
                        return Err(VMInternalError::new(unreachable(
                            "Unexpected signed integer width",
                        )))
                    },
                }
            })
        },
        LayoutKind::Address => {
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

            let elem = layouts.layout(*elem_id).ok_or_else(layout_not_found)?;
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
            let variant_id = *variants
                .get(tag_a as usize)
                .ok_or_else(|| enum_tag_out_of_range(tag_a, variants.len()))?;
            if tag_b as usize >= variants.len() {
                return Err(VMInternalError::new(enum_tag_out_of_range(
                    tag_b,
                    variants.len(),
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
        LayoutKind::Function => {
            // TODO(completeness): function values are not yet supported.
            todo!("function values are not yet supported");
        },
        LayoutKind::Ref => Err(VMInternalError::new(unreachable(
            "Comparison runs on pointee types only",
        ))),
    }
}

/// Deserializes BCS bytes of a value of the given type into the flat in-memory
/// representation at `dst`.
///
/// # Allocating semantics
///
/// May allocate data on the heap, e.g. for vectors. **Never runs GC** and
/// instead fails gracefully with an [`AllocationError`] if there is not enough
/// heap space; it is the caller's responsibility to run GC and retry. Heap
/// exhaustion is a normal error, never undefined behavior.
///
/// # Precondition
///
/// The given type must not be a reference: references are never serialized or
/// deserialized.
///
/// # Safety
///
/// `dst` pointer must be writable for the in-memory size of the given type and
/// outlive the call.
pub unsafe fn deserialize<T: LayoutProvider + ?Sized>(
    layouts: &T,
    heap: &mut Heap,
    ty: InternedType,
    bytes: &[u8],
    dst: *mut u8,
) -> AllocationResult<()> {
    let layout = layouts.layout_by_ty(ty).ok_or_else(layout_not_found)?;

    let mut cursor = 0usize;
    // SAFETY: caller must enforce the safety precondition.
    unsafe { deserialize_impl(layouts, heap, layout, bytes, &mut cursor, dst)? };
    if cursor != bytes.len() {
        return Err(RuntimeError::BCSRemainingInput {
            remaining: bytes.len().saturating_sub(cursor),
        }
        .into());
    }
    Ok(())
}

/// Like [`deserialize`], but returns a [`RuntimeError`] instead of the crate-internal
/// `AllocationError`, so external callers can use it. Does not run GC on heap exhaustion; the caller
/// must ensure the heap has room.
///
/// # Safety
///
/// Same as [`deserialize`]: `dst` must be writable for `ty`'s in-memory size and outlive the call,
/// and `ty` must not be a reference.
pub unsafe fn deserialize_into<T: LayoutProvider + ?Sized>(
    layouts: &T,
    heap: &mut Heap,
    ty: InternedType,
    bytes: &[u8],
    dst: *mut u8,
) -> VMResult<()> {
    // SAFETY: forwarded to the caller.
    unsafe { deserialize(layouts, heap, ty, bytes, dst) }
        .map_err(|e| VMInternalError::new(e.into_runtime_error()))
}

/// # Safety
///
/// `dst` must be writable for `layout.size` bytes.
unsafe fn deserialize_impl<T: LayoutProvider + ?Sized>(
    layouts: &T,
    heap: &mut Heap,
    layout: &ValueLayout,
    bytes: &[u8],
    cursor: &mut usize,
    dst: *mut u8,
) -> AllocationResult<()> {
    // TODO(metering): This walk recurses on struct fields and vector elements; convert it
    // to a non-recursive form to bound stack depth on deeply nested values.
    //
    // If every byte pattern is valid (no padding, no pointers, no `bool`), the
    // value's BCS bytes are exactly its in-memory image. A `bool` is excluded
    // because only `0`/`1` are canonical and a raw copy would skip that check.
    // TODO(correctness): breaks on big-endian hosts. This writes the
    // little-endian BCS bytes verbatim, but the in-memory representation is
    // native-endian, so the two only match on little-endian hosts.
    if layout.all_byte_patterns_valid() {
        let n = layout.size as usize;
        let src = read_slice(bytes, cursor, n)?;
        // SAFETY: caller ensures `n` bytes can be written to `dst` and it is
        // not aliasing `src`.
        unsafe { std::ptr::copy_nonoverlapping(src.as_ptr(), dst, n) };
        return Ok(());
    }

    match &layout.kind {
        LayoutKind::Bool => {
            // BCS encodes a `bool` as a single canonical byte: `0` or `1`. Any
            // other value is rejected rather than blitted into the slot.
            let byte = read_slice(bytes, cursor, 1)?[0];
            if byte > 1 {
                return Err(RuntimeError::BCSInvalidBool { byte }.into());
            }
            // SAFETY: caller ensures one byte can be written to `dst`.
            unsafe { std::ptr::write(dst, byte) };
            Ok(())
        },
        LayoutKind::UnsignedInt | LayoutKind::SignedInt | LayoutKind::Address => {
            Err(unreachable("Integer and address layouts must be handled by fast-path").into())
        },
        LayoutKind::Struct { fields } => {
            for field in fields.iter() {
                let field_layout = layouts.layout(field.id).ok_or_else(layout_not_found)?;
                // SAFETY: value is a valid struct, so all fields lie at `offset`
                // and are within bounds. `dst` is correctly sized so there is
                // enough space to write all fields.
                unsafe {
                    deserialize_impl(
                        layouts,
                        heap,
                        field_layout,
                        bytes,
                        cursor,
                        dst.add(field.offset as usize),
                    )?
                };
            }
            Ok(())
        },
        LayoutKind::Vector {
            elem_id,
            descriptor_id,
        } => {
            let len = read_uleb128_len(bytes, cursor)?;
            if len > bcs::MAX_SEQUENCE_LENGTH as u64 {
                return Err(RuntimeError::BCSSequenceTooLong { len }.into());
            }
            if len == 0 {
                // The empty vector is the null pointer.
                // SAFETY: `dst` has size to write the null pointer as
                // guaranteed by the caller.
                unsafe { write_ptr(dst, 0usize, std::ptr::null()) };
                return Ok(());
            }

            let elem_layout = layouts.layout(*elem_id).ok_or_else(layout_not_found)?;
            let elem_size = elem_layout.size as usize;

            let data_size = (len as usize)
                .checked_mul(elem_size)
                .ok_or(RuntimeError::VecAllocSizeOverflow)?;
            let total_size = data_size
                .checked_add(OBJECT_HEADER_SIZE + VEC_DATA_OFFSET)
                .ok_or(RuntimeError::VecAllocSizeOverflow)?;

            // An OOM here propagates as `AllocationError::OutOfHeapMemory`.
            let vec_ptr = heap_alloc(heap, total_size, *descriptor_id)?;

            // SAFETY: the allocated data must store length at this offset
            // as guaranteed by the allocator.
            unsafe { write_u64(vec_ptr, VEC_LENGTH_OFFSET, len) };

            if elem_layout.all_byte_patterns_valid() {
                // If every element byte pattern is valid, element bytes equal
                // their BCS bytes. A `bool` element is excluded so each byte is
                // validated by the per-element walk below.
                // TODO(correctness): breaks on big-endian hosts, for the same
                // reason as the scalar fast path above: native-endian in-memory
                // bytes equal the little-endian BCS bytes only on little-endian
                // hosts.
                let src = read_slice(bytes, cursor, data_size)?;

                // SAFETY: the vector data region has space to write
                // `data_size` bytes and source has same size.
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        src.as_ptr(),
                        vec_ptr.add(VEC_DATA_OFFSET),
                        data_size,
                    )
                };
            } else {
                for i in 0..len as usize {
                    // SAFETY: ith element lies within the vector data region,
                    // so the pointer is non-null and new pointer points within
                    // the data region.
                    let elem_ptr = unsafe { vec_ptr.add(VEC_DATA_OFFSET + i * elem_size) };
                    // SAFETY: element pointer is a valid value of the given
                    // element layout, as guaranteed by the precondition of the
                    // function.
                    unsafe {
                        deserialize_impl(layouts, heap, elem_layout, bytes, cursor, elem_ptr)?
                    };
                }
            }

            // SAFETY: `dst` has size to write the vector pointer as
            // guaranteed by the caller.
            unsafe { write_ptr(dst, 0usize, vec_ptr) };
            Ok(())
        },
        LayoutKind::FrozenEnum {
            descriptor_id,
            variants,
            max_size_across_variants,
        } => {
            // BCS encodes the variant index as a ULEB128 before the fields.
            let tag = read_uleb128_len(bytes, cursor)?;
            if tag >= variants.len() as u64 {
                return Err(enum_tag_out_of_range(tag, variants.len()).into());
            }

            let variant_layout = layouts
                .layout(variants[tag as usize])
                .ok_or_else(layout_not_found)?;

            let total_size = OBJECT_HEADER_SIZE + *max_size_across_variants as usize;
            let obj_ptr = heap_alloc(heap, total_size, *descriptor_id)?;

            // SAFETY: the allocation has enough size to write the tag.
            unsafe { write_enum_tag(obj_ptr, tag) };

            // SAFETY: variant body lives at the specified offset. The maximum
            // size was allocated so there is enough space to deserialize into.
            unsafe {
                deserialize_impl(
                    layouts,
                    heap,
                    variant_layout,
                    bytes,
                    cursor,
                    obj_ptr.add(ENUM_DATA_OFFSET),
                )?
            };

            // SAFETY: `dst` has space to write the 8-byte enum pointer as
            // guaranteed by the caller.
            unsafe { write_ptr(dst, 0usize, obj_ptr) };
            Ok(())
        },
        LayoutKind::Function => {
            // TODO(completeness): function values are not yet supported.
            todo!("function values are not yet supported");
        },
        LayoutKind::Ref => Err(unreachable("References cannot be deserialized").into()),
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

/// Borrows the next `n` bytes, advancing the cursor. Returns an error if
/// there is not enough bytes to read or the size of the slice overflows.
fn read_slice<'b>(bytes: &'b [u8], cursor: &mut usize, n: usize) -> Result<&'b [u8], RuntimeError> {
    let end = cursor.checked_add(n).ok_or(RuntimeError::BCSEof)?;
    if end > bytes.len() {
        return Err(RuntimeError::BCSEof);
    }
    let slice = &bytes[*cursor..end];
    *cursor = end;
    Ok(slice)
}

// TODO(cleanup): See if we can reuse move-binary-format's uleb128 APIs instead of
// reimplementing the encode/decode here.

/// Writes ULEB128-encoded length data.
fn write_uleb128_len(out: &mut Vec<u8>, mut v: u64) {
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
}

/// Reads ULEB128-encoded length data, advancing the cursor. Returns an error
/// if:
/// - data is not a valid ULEB128,
/// - end of input is unexpectedly reached.
fn read_uleb128_len(bytes: &[u8], cursor: &mut usize) -> Result<u64, RuntimeError> {
    let mut result = 0u64;
    let mut shift = 0u32;
    loop {
        let byte = *bytes.get(*cursor).ok_or(RuntimeError::BCSEof)?;
        *cursor += 1;

        let cur = (byte & 0x7F) as u64;
        // Reject any byte whose payload bits do not survive the shift: either the
        // shift count is out of range, or the high bits would be truncated (e.g.
        // a terminal `0x02` on the 10th byte where `shift == 63`).
        if shift >= 64 || (cur << shift) >> shift != cur {
            return Err(RuntimeError::BCSInvalidUleb);
        }
        result |= cur << shift;
        if byte & 0x80 == 0 {
            // Reject a non-minimal encoding (a trailing zero continuation).
            if byte == 0 && shift != 0 {
                return Err(RuntimeError::BCSInvalidUleb);
            }
            return Ok(result);
        }
        shift += 7;
    }
}

/// Invariant violation error when layout is not available during value walk.
fn layout_not_found() -> RuntimeError {
    RuntimeError::InvariantViolation(RuntimeInvariantViolation::ValueLayoutNotFound)
}

/// An invariant violation for an unreachable state.
fn unreachable(message: &str) -> RuntimeError {
    RuntimeError::InvariantViolation(RuntimeInvariantViolation::Unreachable(message.to_string()))
}

/// Invariant violation when an enum tag does not name a variant, whether read
/// from an in-memory value or decoded from BCS. A well-formed enum's tag is
/// always in range, so an out-of-range tag signals corruption, not a legitimate
/// runtime condition.
fn enum_tag_out_of_range(tag: u64, variant_count: usize) -> RuntimeError {
    RuntimeError::InvariantViolation(RuntimeInvariantViolation::EnumTagOutOfRange {
        tag,
        variant_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::heap::AllocationError;
    use mono_move_core::{
        align_up_u32,
        types::U64_TY,
        value_layout::{BOOL_LAYOUT_ID, U16_LAYOUT_ID, U64_LAYOUT_ID, U8_LAYOUT_ID},
        DescriptorId, FieldValueLayout, LayoutFlags, LayoutId, ValueLayoutTable,
    };
    use serde::Serialize;
    use std::mem::{offset_of, size_of};

    fn ptr<T>(x: &T) -> *const u8 {
        x as *const T as *const u8
    }

    fn vector_layout(elem_id: LayoutId) -> ValueLayout {
        ValueLayout::vector(elem_id, DescriptorId(2))
    }

    #[test]
    fn deserialize_empty_vector_is_null() {
        let mut heap = Heap::new(4096);
        let table = ValueLayoutTable::new();
        let layout = vector_layout(U8_LAYOUT_ID);
        let bytes = [0u8]; // ULEB len 0.
        let mut slot = 0u64;
        let mut cursor = 0;
        unsafe {
            deserialize_impl(
                &table,
                &mut heap,
                &layout,
                &bytes,
                &mut cursor,
                &mut slot as *mut u64 as *mut u8,
            )
            .unwrap()
        };
        assert_eq!(cursor, 1);
        assert_eq!(slot, 0, "empty vector is the null pointer");
    }

    #[test]
    fn deserialize_out_of_heap_memory_errors() {
        let mut table = ValueLayoutTable::new();
        let vid = table.push(U64_TY, vector_layout(U64_LAYOUT_ID));
        let layout = table.layout(vid).unwrap();
        // The vector for 1000 u64s far exceeds this heap, so allocation fails.
        let bytes = bcs::to_bytes(&vec![0u64; 1000]).unwrap();
        let mut heap = Heap::new(128);
        let mut slot = 0u64;
        let mut cursor = 0;
        let result = unsafe {
            deserialize_impl(
                &table,
                &mut heap,
                layout,
                &bytes,
                &mut cursor,
                &mut slot as *mut u64 as *mut u8,
            )
        };
        assert!(matches!(
            result,
            Err(AllocationError::OutOfHeapMemory { .. })
        ));
    }

    #[test]
    fn read_uleb128_len_valid() {
        // The max payload on the 10th byte is bit 63, so `0x01` there encodes
        // `2^63` and nine `0x7F` payload bytes plus `0x01` encode `u64::MAX`.
        let cases: &[(&[u8], u64)] = &[
            (&[0x00], 0),
            (&[0x01], 1),
            (&[0x7F], 127),
            (&[0x80, 0x01], 128),
            (&[0xFF, 0x01], 255),
            (
                &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01],
                u64::MAX,
            ),
            (
                &[0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01],
                1u64 << 63,
            ),
        ];
        for (bytes, expected) in cases {
            let mut cursor = 0;
            assert_eq!(read_uleb128_len(bytes, &mut cursor).unwrap(), *expected);
            assert_eq!(cursor, bytes.len());
        }
    }

    #[test]
    fn read_uleb128_len_rejects_overflow_on_last_byte() {
        // The 10th byte sits at `shift == 63`; a payload above bit 0 (here
        // `0x02`) would be truncated, so it must be rejected, not accepted as 0.
        let bytes = [0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02];
        let mut cursor = 0;
        assert!(matches!(
            read_uleb128_len(&bytes, &mut cursor),
            Err(RuntimeError::BCSInvalidUleb)
        ));
    }

    #[test]
    fn read_uleb128_len_rejects_too_many_bytes() {
        // An 11th byte pushes `shift` to 70, past the u64 width.
        let bytes = [
            0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01,
        ];
        let mut cursor = 0;
        assert!(matches!(
            read_uleb128_len(&bytes, &mut cursor),
            Err(RuntimeError::BCSInvalidUleb)
        ));
    }

    #[test]
    fn read_uleb128_len_rejects_non_minimal() {
        // A trailing zero continuation byte is a non-canonical encoding of 0.
        let bytes = [0x80, 0x00];
        let mut cursor = 0;
        assert!(matches!(
            read_uleb128_len(&bytes, &mut cursor),
            Err(RuntimeError::BCSInvalidUleb)
        ));
    }

    #[test]
    fn read_uleb128_len_eof() {
        // The continuation bit is set but no further byte follows.
        let bytes = [0x80, 0x80];
        let mut cursor = 0;
        assert!(matches!(
            read_uleb128_len(&bytes, &mut cursor),
            Err(RuntimeError::BCSEof)
        ));
    }

    #[test]
    fn deserialize_rejects_trailing_bytes() {
        let mut table = ValueLayoutTable::new();
        table.push(U64_TY, ValueLayout::u64());
        let mut heap = Heap::new(4096);

        let mut bytes = bcs::to_bytes(&7u64).unwrap();
        bytes.push(0xAB); // Trailing byte after a complete value.
        let mut slot = 0u64;
        let result = unsafe {
            deserialize(
                &table,
                &mut heap,
                U64_TY,
                &bytes,
                &mut slot as *mut u64 as *mut u8,
            )
        };
        assert!(matches!(
            result,
            Err(AllocationError::RuntimeError(
                RuntimeError::BCSRemainingInput { remaining: 1 }
            ))
        ));
    }

    #[test]
    fn deserialize_bool_canonical() {
        let table = ValueLayoutTable::new();
        let layout = table.layout(BOOL_LAYOUT_ID).unwrap();
        // A bool reaches the per-byte walk, not the blit fast path.
        assert!(layout.has_no_pointers_no_padding());
        assert!(!layout.all_byte_patterns_valid());

        let mut heap = Heap::new(64);
        for byte in [0u8, 1] {
            let mut slot = 0xFFu8;
            let mut cursor = 0;
            unsafe {
                deserialize_impl(&table, &mut heap, layout, &[byte], &mut cursor, &mut slot)
                    .unwrap()
            };
            assert_eq!(slot, byte);
            assert_eq!(cursor, 1);
        }
    }

    #[test]
    fn deserialize_rejects_non_canonical_bool() {
        let table = ValueLayoutTable::new();
        let layout = table.layout(BOOL_LAYOUT_ID).unwrap();
        let mut heap = Heap::new(64);
        let mut slot = 0u8;
        let mut cursor = 0;
        let result =
            unsafe { deserialize_impl(&table, &mut heap, layout, &[2u8], &mut cursor, &mut slot) };
        assert!(matches!(
            result,
            Err(AllocationError::RuntimeError(
                RuntimeError::BCSInvalidBool { byte: 2 }
            ))
        ));
    }

    #[test]
    fn deserialize_rejects_non_canonical_bool_in_vector() {
        let mut table = ValueLayoutTable::new();
        let vid = table.push(U64_TY, vector_layout(BOOL_LAYOUT_ID));
        let layout = table.layout(vid).unwrap();
        let mut heap = Heap::new(4096);
        // Length 2, then a canonical `1` and a non-canonical `2`.
        let bytes = [0x02u8, 0x01, 0x02];
        let mut slot = 0u64;
        let mut cursor = 0;
        let result = unsafe {
            deserialize_impl(
                &table,
                &mut heap,
                layout,
                &bytes,
                &mut cursor,
                &mut slot as *mut u64 as *mut u8,
            )
        };
        assert!(matches!(
            result,
            Err(AllocationError::RuntimeError(
                RuntimeError::BCSInvalidBool { byte: 2 }
            ))
        ));
    }

    #[test]
    fn deserialize_rejects_non_canonical_bool_in_struct() {
        // `{u8, bool}`: no padding (BCS 2 == size 2), so blittable for
        // serialize/equals, but the bool field keeps it off the deserialize
        // fast path so the bad byte is caught.
        let mut table = ValueLayoutTable::new();
        let layout = build_struct_layout(&table, 2, vec![(0, U8_LAYOUT_ID), (1, BOOL_LAYOUT_ID)]);
        assert!(layout.has_no_pointers_no_padding());
        assert!(!layout.all_byte_patterns_valid());
        let id = table.push(U64_TY, layout);
        let layout = table.layout(id).unwrap();

        let mut heap = Heap::new(64);
        // u8 = 5, then a non-canonical bool byte.
        let bytes = [0x05u8, 0x02];
        let mut slot = [0u8; 2];
        let mut cursor = 0;
        let result = unsafe {
            deserialize_impl(
                &table,
                &mut heap,
                layout,
                &bytes,
                &mut cursor,
                slot.as_mut_ptr(),
            )
        };
        assert!(matches!(
            result,
            Err(AllocationError::RuntimeError(
                RuntimeError::BCSInvalidBool { byte: 2 }
            ))
        ));
    }

    /// Builds a struct layout from field offsets/ids and the in-memory size,
    /// deriving the const BCS size and no-padding flag as the specializer does.
    fn build_struct_layout(
        table: &ValueLayoutTable,
        size: u32,
        fields: Vec<(u32, LayoutId)>,
    ) -> ValueLayout {
        let mut const_total = 0u64;
        let mut data_dependent = false;
        let mut all_bytes_valid = true;
        for &(_, id) in &fields {
            let child = table.layout(id).unwrap();
            match child.fixed_serialized_size() {
                Some(n) => const_total += n as u64,
                None => data_dependent = true,
            }
            all_bytes_valid &= child.all_byte_patterns_valid();
        }
        let const_bcs = (!data_dependent).then_some(const_total as u32);
        let mut flags = LayoutFlags::empty();
        if const_bcs == Some(size) {
            flags |= LayoutFlags::NO_POINTERS_NO_PADDING;
            if all_bytes_valid {
                flags |= LayoutFlags::ALL_BYTE_PATTERNS_VALID;
            }
        }
        let field_layouts = fields
            .into_iter()
            .map(|(offset, id)| FieldValueLayout { offset, id })
            .collect::<Vec<_>>()
            .into_boxed_slice();
        ValueLayout::struct_layout(size, 8, const_bcs, flags, field_layouts)
    }

    /// Checks the walks for a fixed-size struct against the Rust oracle:
    /// `bcs::to_bytes` for serialization and size, and derived `Ord`/`Eq` for
    /// pairwise comparison and equality.
    ///
    /// # Safety
    ///
    /// Every value must match the layout at `id`, and `size` must be that
    /// layout's in-memory size.
    unsafe fn check_struct<S: Serialize + Ord>(
        table: &ValueLayoutTable,
        id: LayoutId,
        values: &[S],
        size: usize,
    ) {
        let layout = table.layout(id).unwrap();
        let bcs_len = bcs::to_bytes(&values[0]).unwrap().len();

        // Const size is the packed BCS size; blittable iff that equals the
        // in-memory size.
        assert_eq!(layout.fixed_serialized_size(), Some(bcs_len as u32));
        assert_eq!(layout.has_no_pointers_no_padding(), bcs_len == size);

        for x in values {
            let x_bcs = bcs::to_bytes(x).unwrap();
            // A fixed-size struct encodes to the same length for every value.
            assert_eq!(x_bcs.len(), bcs_len);

            // Serialize matches bcs; a wrongly-blittable padded struct would
            // memcpy its padding and diverge here.
            let mut out = vec![];
            unsafe { serialize_impl(table, ptr(x), layout, &mut out).unwrap() };
            assert_eq!(out, x_bcs);

            // Round-trip through the packed bytes, not `dst` directly: the
            // field walk leaves padding bytes in `dst` untouched.
            let mut heap = Heap::new(128);
            let mut dst = vec![0u8; size];
            let mut cursor = 0;
            unsafe {
                deserialize_impl(
                    table,
                    &mut heap,
                    layout,
                    &x_bcs,
                    &mut cursor,
                    dst.as_mut_ptr(),
                )
                .unwrap()
            };
            assert_eq!(cursor, x_bcs.len());
            let mut reser = vec![];
            unsafe { serialize_impl(table, dst.as_ptr(), layout, &mut reser).unwrap() };
            assert_eq!(reser, x_bcs);
        }

        // Pairwise ordering and equality match Rust's derived Ord/Eq (repr(C)
        // declaration order is offset order, which both sides walk).
        for x in values {
            for y in values {
                unsafe {
                    assert_eq!(compare_impl(table, ptr(x), ptr(y), id).unwrap(), x.cmp(y));
                    assert_eq!(equals_impl(table, ptr(x), ptr(y), id).unwrap(), x == y);
                }
            }
        }
    }

    #[test]
    fn test_struct_padded() {
        #[repr(C)]
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        struct S {
            a: u8,
            b: u64,
        }

        let mut table = ValueLayoutTable::new();
        let layout = build_struct_layout(&table, size_of::<S>() as u32, vec![
            (offset_of!(S, a) as u32, U8_LAYOUT_ID),
            (offset_of!(S, b) as u32, U64_LAYOUT_ID),
        ]);
        // In-memory 16, BCS 9 (7 bytes padding), so not blittable.
        assert_eq!(layout.fixed_serialized_size(), Some(9));
        assert!(!layout.has_no_pointers_no_padding());

        let id = table.push(U64_TY, layout);
        let values = [
            S { a: 0, b: 0 },
            S { a: 0, b: 1 },
            S { a: 1, b: 0 },
            S { a: 1, b: 0 },
            S {
                a: u8::MAX,
                b: u64::MAX,
            },
        ];
        unsafe { check_struct(&table, id, &values, size_of::<S>()) };
    }

    #[test]
    fn test_struct_blittable() {
        #[repr(C)]
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        struct S {
            a: u64,
            b: u64,
        }

        let mut table = ValueLayoutTable::new();
        let layout = build_struct_layout(&table, size_of::<S>() as u32, vec![
            (offset_of!(S, a) as u32, U64_LAYOUT_ID),
            (offset_of!(S, b) as u32, U64_LAYOUT_ID),
        ]);
        // No padding: BCS 16 equals in-memory 16, so blittable.
        assert_eq!(layout.fixed_serialized_size(), Some(16));
        assert!(layout.has_no_pointers_no_padding());

        let id = table.push(U64_TY, layout);
        let values = [
            S { a: 0, b: 0 },
            S { a: 0, b: 1 },
            S { a: 1, b: 0 },
            S { a: 1, b: 0 },
            S {
                a: u64::MAX,
                b: u64::MAX,
            },
        ];
        unsafe { check_struct(&table, id, &values, size_of::<S>()) };
    }

    #[test]
    fn test_struct_three_fields() {
        // `{u16, u8, u64}`: a@0, b@2, c@8 (two padding gaps), size 16, BCS 11.
        #[repr(C)]
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        struct S {
            a: u16,
            b: u8,
            c: u64,
        }

        let mut table = ValueLayoutTable::new();
        let layout = build_struct_layout(&table, size_of::<S>() as u32, vec![
            (offset_of!(S, a) as u32, U16_LAYOUT_ID),
            (offset_of!(S, b) as u32, U8_LAYOUT_ID),
            (offset_of!(S, c) as u32, U64_LAYOUT_ID),
        ]);
        assert_eq!(layout.fixed_serialized_size(), Some(11));
        assert!(!layout.has_no_pointers_no_padding());

        let id = table.push(U64_TY, layout);
        let values = [
            S { a: 0, b: 0, c: 0 },
            S { a: 0, b: 0, c: 1 },
            S { a: 0, b: 1, c: 0 },
            S { a: 1, b: 0, c: 0 },
            S { a: 0, b: 1, c: 0 },
            S {
                a: u16::MAX,
                b: u8::MAX,
                c: u64::MAX,
            },
        ];
        unsafe { check_struct(&table, id, &values, size_of::<S>()) };
    }

    #[test]
    fn test_struct_nested() {
        #[repr(C)]
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        struct Inner {
            a: u64,
            b: u64,
        }
        #[repr(C)]
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        struct Outer {
            x: Inner,
            y: u8,
        }

        let mut table = ValueLayoutTable::new();
        let inner = build_struct_layout(&table, size_of::<Inner>() as u32, vec![
            (offset_of!(Inner, a) as u32, U64_LAYOUT_ID),
            (offset_of!(Inner, b) as u32, U64_LAYOUT_ID),
        ]);
        let inner_id = table.push(U64_TY, inner);

        let outer = build_struct_layout(&table, size_of::<Outer>() as u32, vec![
            (offset_of!(Outer, x) as u32, inner_id),
            (offset_of!(Outer, y) as u32, U8_LAYOUT_ID),
        ]);
        // Outer is BCS 17, size 24 (trailing pad): a blittable child (Inner)
        // does not make the parent blittable.
        assert_eq!(outer.fixed_serialized_size(), Some(17));
        assert!(!outer.has_no_pointers_no_padding());

        let id = table.push(U64_TY, outer);
        let values = [
            Outer {
                x: Inner { a: 0, b: 0 },
                y: 0,
            },
            Outer {
                x: Inner { a: 0, b: 0 },
                y: 1,
            },
            Outer {
                x: Inner { a: 0, b: 1 },
                y: 0,
            },
            Outer {
                x: Inner { a: 1, b: 0 },
                y: 0,
            },
            Outer {
                x: Inner { a: 0, b: 0 },
                y: 1,
            },
        ];
        unsafe { check_struct(&table, id, &values, size_of::<Outer>()) };
    }

    /// Like [`check_struct`], but builds the value by deserializing its bcs
    /// rather than pointing at the Rust value, so it also covers types that own
    /// heap boxes (vectors, and structs containing them) whose in-memory layout
    /// differs from Rust's.
    ///
    /// # Safety
    ///
    /// `id` and `size` must be the layout and in-memory size matching `S`.
    unsafe fn check_roundtrip<S: Serialize + Ord>(
        table: &ValueLayoutTable,
        id: LayoutId,
        size: usize,
        values: &[S],
    ) {
        let layout = table.layout(id).unwrap();
        let mut heap = Heap::new(8192);
        // Each buffer holds the deserialized value; its boxes live in `heap`.
        let mut bufs: Vec<Vec<u8>> = Vec::with_capacity(values.len());
        for v in values {
            let bytes = bcs::to_bytes(v).unwrap();
            let mut dst = vec![0u8; size];
            let mut cursor = 0;
            unsafe {
                deserialize_impl(
                    table,
                    &mut heap,
                    layout,
                    &bytes,
                    &mut cursor,
                    dst.as_mut_ptr(),
                )
                .unwrap()
            };
            assert_eq!(cursor, bytes.len());

            // Re-serializing the deserialized value reproduces its bcs.
            let mut out = vec![];
            unsafe { serialize_impl(table, dst.as_ptr(), layout, &mut out).unwrap() };
            assert_eq!(out, bytes);

            bufs.push(dst);
        }

        // Compare and equals match the value's `Ord`/`Eq` across every pair.
        for (i, vi) in values.iter().enumerate() {
            for (j, vj) in values.iter().enumerate() {
                let (pi, pj) = (bufs[i].as_ptr(), bufs[j].as_ptr());
                unsafe {
                    assert_eq!(compare_impl(table, pi, pj, id).unwrap(), vi.cmp(vj));
                    assert_eq!(equals_impl(table, pi, pj, id).unwrap(), vi == vj);
                }
            }
        }
    }

    #[test]
    fn test_vector_u64() {
        let mut table = ValueLayoutTable::new();
        let vid = table.push(U64_TY, vector_layout(U64_LAYOUT_ID));
        assert_eq!(table.layout(vid).unwrap().fixed_serialized_size(), None);
        let values: [Vec<u64>; 7] = [
            vec![],
            vec![1],
            vec![1, 2],
            vec![1, 2, 3],
            vec![1, 3],
            vec![2],
            vec![1, 2],
        ];
        unsafe { check_roundtrip(&table, vid, 8, &values) };
    }

    #[test]
    fn test_vector_u8() {
        let mut table = ValueLayoutTable::new();
        let vid = table.push(U64_TY, vector_layout(U8_LAYOUT_ID));
        let values = vec![
            vec![],
            vec![0u8],
            vec![0u8, 0],
            vec![255u8],
            vec![1u8, 2, 3],
            vec![7u8; 200], // > 127 forces a multi-byte ULEB length.
        ];
        unsafe { check_roundtrip(&table, vid, 8, &values) };
    }

    #[test]
    fn test_vector_nested() {
        let mut table = ValueLayoutTable::new();
        // `vector<vector<u64>>`: the element is a vector pointer (not
        // blittable), so the walks recurse per element.
        let inner_id = table.push(U64_TY, vector_layout(U64_LAYOUT_ID));
        let vid = table.push(U64_TY, vector_layout(inner_id));
        let values: [Vec<Vec<u64>>; 6] = [
            vec![],
            vec![vec![]],
            vec![vec![1]],
            vec![vec![1], vec![2]],
            vec![vec![1, 2]],
            vec![vec![1], vec![2]],
        ];
        unsafe { check_roundtrip(&table, vid, 8, &values) };
    }

    #[test]
    fn test_vector_of_struct() {
        // Element is a padded (non-blittable) struct, so the vector recurses
        // per element into the struct field walk.
        #[repr(C)]
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        struct Kv {
            k: u8,
            v: u64,
        }

        let mut table = ValueLayoutTable::new();
        let kv_layout = build_struct_layout(&table, size_of::<Kv>() as u32, vec![
            (offset_of!(Kv, k) as u32, U8_LAYOUT_ID),
            (offset_of!(Kv, v) as u32, U64_LAYOUT_ID),
        ]);
        let kv_id = table.push(U64_TY, kv_layout);
        let vid = table.push(U64_TY, vector_layout(kv_id));

        let values: [Vec<Kv>; 5] = [
            vec![],
            vec![Kv { k: 1, v: 2 }],
            vec![Kv { k: 1, v: 2 }, Kv { k: 3, v: 4 }],
            vec![Kv { k: 1, v: 9 }],
            vec![Kv { k: 1, v: 2 }],
        ];
        unsafe { check_roundtrip(&table, vid, 8, &values) };
    }

    #[test]
    fn test_struct_of_vector() {
        // `{u64, vector<u64>}`: in-memory size 16 (u64 + box pointer), narrower
        // than the Rust oracle's `Vec` field, so build the layout explicitly
        // and use `Bag` only as the oracle (never point at it).
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        struct Bag {
            id: u64,
            items: Vec<u64>,
        }

        let mut table = ValueLayoutTable::new();
        let vec_id = table.push(U64_TY, vector_layout(U64_LAYOUT_ID));
        let bag_layout = build_struct_layout(&table, 16, vec![(0, U64_LAYOUT_ID), (8, vec_id)]);
        let id = table.push(U64_TY, bag_layout);
        // The vector field makes the struct data-dependent and not blittable.
        assert_eq!(table.layout(id).unwrap().fixed_serialized_size(), None);
        assert!(!table.layout(id).unwrap().has_no_pointers_no_padding());

        let values: [Bag; 5] = [
            Bag {
                id: 1,
                items: vec![],
            },
            Bag {
                id: 1,
                items: vec![7],
            },
            Bag {
                id: 2,
                items: vec![],
            },
            Bag {
                id: 1,
                items: vec![7, 8],
            },
            Bag {
                id: 1,
                items: vec![7],
            },
        ];
        unsafe { check_roundtrip(&table, id, 16, &values) };
    }

    /// Builds and pushes an enum layout into `table`: one struct layout per
    /// variant (via [`build_struct_layout`]), then the enum layout itself.
    /// Each variant is `(in_memory_size, fields)`, with `fields` as
    /// `(data_region_relative_offset, layout_id)` pairs. The heap payload size
    /// is the 8-byte tag plus the widest variant region, rounded up to 8.
    fn build_enum_layout(
        table: &mut ValueLayoutTable,
        variants: Vec<(u32, Vec<(u32, LayoutId)>)>,
    ) -> LayoutId {
        let mut variant_ids = Vec::with_capacity(variants.len());
        let mut max_data = 0u32;
        for (size, fields) in variants {
            max_data = max_data.max(size);
            let layout = build_struct_layout(table, size, fields);
            variant_ids.push(table.push(U64_TY, layout));
        }
        let max_size_across_variants = align_up_u32(8 + max_data, 8);
        let layout = ValueLayout::frozen_enum(
            DescriptorId(3),
            variant_ids.into_boxed_slice(),
            max_size_across_variants,
        );
        table.push(U64_TY, layout)
    }

    #[test]
    fn test_enum_unit_and_tuple_variants() {
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        enum E {
            A,
            B(u64),
            C(u8, u64),
        }

        let mut table = ValueLayoutTable::new();
        let eid = build_enum_layout(&mut table, vec![
            (0, vec![]),
            (8, vec![(0, U64_LAYOUT_ID)]),
            (16, vec![(0, U8_LAYOUT_ID), (8, U64_LAYOUT_ID)]),
        ]);
        let values = [
            E::A,
            E::B(0),
            E::B(7),
            E::C(1, 2),
            E::C(1, 3),
            E::C(2, 0),
            E::A,
        ];
        unsafe { check_roundtrip(&table, eid, 8, &values) };
    }

    #[test]
    fn test_enum_with_vector_variant() {
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        enum E {
            Empty,
            Items(Vec<u64>),
        }

        let mut table = ValueLayoutTable::new();
        let vec_id = table.push(U64_TY, vector_layout(U64_LAYOUT_ID));
        let eid = build_enum_layout(&mut table, vec![(0, vec![]), (8, vec![(0, vec_id)])]);
        let values = [
            E::Empty,
            E::Items(vec![]),
            E::Items(vec![1]),
            E::Items(vec![1, 2]),
            E::Items(vec![2]),
            E::Items(vec![1]),
        ];
        unsafe { check_roundtrip(&table, eid, 8, &values) };
    }

    #[test]
    fn test_enum_as_struct_field() {
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        enum Inner {
            X,
            Y(u64),
        }
        #[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
        struct Outer {
            id: u64,
            e: Inner,
        }

        let mut table = ValueLayoutTable::new();
        let inner_eid =
            build_enum_layout(&mut table, vec![(0, vec![]), (8, vec![(0, U64_LAYOUT_ID)])]);
        let outer = build_struct_layout(&table, 16, vec![(0, U64_LAYOUT_ID), (8, inner_eid)]);
        let oid = table.push(U64_TY, outer);
        let values = [
            Outer { id: 1, e: Inner::X },
            Outer {
                id: 1,
                e: Inner::Y(0),
            },
            Outer {
                id: 1,
                e: Inner::Y(5),
            },
            Outer { id: 2, e: Inner::X },
            Outer { id: 1, e: Inner::X },
        ];
        unsafe { check_roundtrip(&table, oid, 16, &values) };
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use mono_move_core::{
        types::{
            ADDRESS_TY, BOOL_TY, I128_TY, I16_TY, I256_TY, I32_TY, I64_TY, I8_TY, U128_TY, U16_TY,
            U256_TY, U32_TY, U64_TY, U8_TY,
        },
        ValueLayoutTable,
    };
    use move_core_types::{
        account_address::AccountAddress,
        int256::{I256, U256},
    };
    use proptest::prelude::*;

    macro_rules! prop_primitive {
        ($name:ident, $t:ty, $ty:expr, $strat:expr) => {
            proptest! {
                #[test]
                fn $name(x in $strat, y in $strat) {
                    let table = ValueLayoutTable::new();
                    let mut heap = Heap::new(128);
                    let size = std::mem::size_of::<$t>();
                    let bytes = bcs::to_bytes(&x).unwrap();
                    prop_assert_eq!(bytes.len(), size);

                    // Constant serialized size is the fixed width.
                    prop_assert_eq!(
                        fixed_serialized_size(&table, $ty).unwrap(),
                        Some(size)
                    );

                    let px = &x as *const $t as *const u8;
                    let py = &y as *const $t as *const u8;

                    // Serialize matches the BCS encoding.
                    let out = unsafe { serialize(&table, px, $ty).unwrap() };
                    prop_assert_eq!(&out, &bytes);
                    prop_assert_eq!(
                        unsafe { serialized_size(&table, px, $ty).unwrap() },
                        bytes.len()
                    );

                    // Deserialize reproduces the value's in-memory bytes.
                    let mut dst = vec![0u8; size];
                    unsafe {
                        deserialize(&table, &mut heap, $ty, &bytes, dst.as_mut_ptr()).unwrap()
                    };
                    let x_bytes = unsafe { std::slice::from_raw_parts(px, size) };
                    prop_assert_eq!(dst.as_slice(), x_bytes);

                    // Compare and equals match Rust's ordering and equality.
                    prop_assert_eq!(unsafe { compare(&table, px, py, $ty).unwrap() }, x.cmp(&y));
                    prop_assert_eq!(unsafe { equals(&table, px, py, $ty).unwrap() }, x == y);
                    let eq_self = unsafe { equals(&table, px, px, $ty).unwrap() };
                    prop_assert!(eq_self);
                }
            }
        };
    }

    macro_rules! unsigned_strategy {
        ($t:ty) => {
            prop_oneof![Just(0 as $t), Just(<$t>::MAX), any::<$t>()]
        };
    }

    macro_rules! signed_strategy {
        ($t:ty) => {
            prop_oneof![
                Just(0 as $t),
                Just(-1 as $t),
                Just(<$t>::MIN),
                Just(<$t>::MAX),
                any::<$t>()
            ]
        };
    }

    prop_primitive!(prop_bool, bool, BOOL_TY, prop_oneof![
        Just(false),
        Just(true)
    ]);
    prop_primitive!(prop_u8, u8, U8_TY, unsigned_strategy!(u8));
    prop_primitive!(prop_u16, u16, U16_TY, unsigned_strategy!(u16));
    prop_primitive!(prop_u32, u32, U32_TY, unsigned_strategy!(u32));
    prop_primitive!(prop_u64, u64, U64_TY, unsigned_strategy!(u64));
    prop_primitive!(prop_u128, u128, U128_TY, unsigned_strategy!(u128));
    prop_primitive!(prop_i8, i8, I8_TY, signed_strategy!(i8));
    prop_primitive!(prop_i16, i16, I16_TY, signed_strategy!(i16));
    prop_primitive!(prop_i32, i32, I32_TY, signed_strategy!(i32));
    prop_primitive!(prop_i64, i64, I64_TY, signed_strategy!(i64));
    prop_primitive!(prop_i128, i128, I128_TY, signed_strategy!(i128));
    prop_primitive!(prop_u256, U256, U256_TY, prop_oneof![
        Just(U256::MIN),
        Just(U256::MAX),
        any::<[u8; 32]>().prop_map(U256::from_le_bytes)
    ]);
    prop_primitive!(prop_i256, I256, I256_TY, prop_oneof![
        Just(I256::MIN),
        Just(I256::MAX),
        Just(I256::from_le_bytes([0xFF; 32])),
        any::<[u8; 32]>().prop_map(I256::from_le_bytes)
    ]);
    prop_primitive!(prop_address, AccountAddress, ADDRESS_TY, prop_oneof![
        Just(AccountAddress::new([0; 32])),
        Just(AccountAddress::new([0xFF; 32])),
        any::<[u8; 32]>().prop_map(AccountAddress::new)
    ]);
}
