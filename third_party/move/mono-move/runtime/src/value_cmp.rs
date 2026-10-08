// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Structural equality and ordering of two VM values, driven by their layout.
//
// TODO(correctness): the comparison fast paths assume a little-endian host.

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation},
    memory::{read_enum_tag, read_ptr, read_vec_len},
    types::VEC_DATA_OFFSET,
    value_conv::bcs::{
        capture_blob, capture_slots, captured_fields, closure_parts, decode_captures,
        emit_closure_captures, materialized_layout_id, ClosureParts,
    },
};
use mono_move_core::{
    interner::{type_tag_of, view_module_id, FunctionRef},
    types::{view_name, view_type_list, InternedType},
    LayoutId, LayoutKind, LayoutProvider, VMInternalError, VMResult, ENUM_DATA_OFFSET,
};
use move_core_types::{
    function::MoveClosure,
    int256::{I256, U256},
    language_storage::TypeTag,
    value::{MoveStruct, MoveStructLayout, MoveTypeLayout, MoveValue},
};
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
        // SAFETY: a function slot holds a closure pointer, and the caller
        // guarantees its heap objects are live.
        LayoutKind::Function => unsafe { closures_equal(layouts, a, b) },
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
/// 6. Function values are not ordered: reaching one is an error.
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
        // Function values support equality but not ordering.
        LayoutKind::Function => Err(VMInternalError::new(RuntimeError::Unsupported(
            "function values cannot be ordered",
        ))),
        LayoutKind::Ref => Err(VMInternalError::new(RuntimeError::InvariantViolation(
            RuntimeInvariantViolation::Unreachable(
                "Comparison runs on pointee types only".to_string(),
            ),
        ))),
    }
}

// ---------------------------------------------------------------------------
// Function values
// ---------------------------------------------------------------------------
//
// A closure's captures live in one of two forms: flat values laid out by
// `PackClosure`, or the wire bytes a closure read from storage was built from.
// Comparing a flat pair against a wire pair means putting them in one form, and
// the flat side is the one that converts: emitting its wire bytes needs only
// the layout table, while decoding the wire side would need the target loaded
// and lowered, giving comparison a failure class V1 does not have.
//
// TODO(security, metering): a mixed pair re-emits the flat side on every
// comparison. Check that the `Eq` gas formula scales with capture size, and
// reuse a per-worker scratch buffer.

/// Structural equality of two function values.
///
/// # Safety
///
/// `a` and `b` must point to fully initialized function values whose reachable
/// heap objects stay live throughout the call.
unsafe fn closures_equal<T: LayoutProvider + ?Sized>(
    layouts: &T,
    a: *const u8,
    b: *const u8,
) -> VMResult<bool> {
    // SAFETY: forwarded from this function's contract.
    let (a, b) = unsafe { (closure_parts(a)?, closure_parts(b)?) };
    if a.mask != b.mask || !same_target(&a, &b)? {
        return Ok(false);
    }
    let count = a.captured_count();
    if count == 0 {
        return Ok(true);
    }

    // SAFETY: a closure capturing anything has a live captured-data object.
    let ids = unsafe {
        (
            materialized_layout_id(a.captured_data)?,
            materialized_layout_id(b.captured_data)?,
        )
    };
    if let (Some(id_a), Some(id_b)) = ids {
        if id_a == id_b {
            // One layout for both, so the captures compare in place.
            let fields = captured_fields(layouts, id_a, count)?;
            for slot in capture_slots(layouts, fields) {
                let (id, _, offset) = slot?;
                // SAFETY: both captures lie within their values region and are
                // described by the layout they were published under.
                let eq = unsafe {
                    equals_impl(
                        layouts,
                        a.captured_data.add(offset),
                        b.captured_data.add(offset),
                        id,
                    )?
                };
                if !eq {
                    return Ok(false);
                }
            }
            return Ok(true);
        }
    }

    // SAFETY: forwarded.
    let (bytes_a, bytes_b) = unsafe { (capture_bytes(layouts, &a)?, capture_bytes(layouts, &b)?) };
    // BCS is canonical, and each tail carries the layouts it was written under,
    // so identical bytes are identical values. A closure read from storage and
    // never called still holds the bytes it came from, which makes filtering an
    // unchanged resource write a memcmp.
    if bytes_a.as_ref() == bytes_b.as_ref() {
        return Ok(true);
    }
    Ok(compare_captures(bytes_a.as_ref(), bytes_b.as_ref(), count)?.is_eq())
}

/// Whether two closures call the same function with the same type arguments.
/// Interning is canonical, so matching targets share one `FunctionRef`.
fn same_target(a: &ClosureParts, b: &ClosureParts) -> VMResult<bool> {
    Ok(std::ptr::eq(a.func_ref, b.func_ref) || compare_func_refs(a.func_ref, b.func_ref)?.is_eq())
}

/// A closure's captures as wire bytes: the stored blob when it has one, else
/// freshly emitted from the flat values.
enum CaptureBytes<'a> {
    Stored(&'a [u8]),
    Emitted(Vec<u8>),
}

impl AsRef<[u8]> for CaptureBytes<'_> {
    fn as_ref(&self) -> &[u8] {
        match self {
            CaptureBytes::Stored(bytes) => bytes,
            CaptureBytes::Emitted(bytes) => bytes,
        }
    }
}

/// # Safety
///
/// `parts.captured_data` must be non-null and point to a live captured-data
/// object that stays live for the lifetime of the result.
unsafe fn capture_bytes<'a, T: LayoutProvider + ?Sized>(
    layouts: &T,
    parts: &ClosureParts<'a>,
) -> VMResult<CaptureBytes<'a>> {
    // SAFETY: forwarded from this function's contract.
    if unsafe { materialized_layout_id(parts.captured_data)? }.is_none() {
        return Ok(CaptureBytes::Stored(unsafe {
            capture_blob(parts.captured_data)
        }));
    }
    let mut out = vec![];
    unsafe { emit_closure_captures(layouts, parts, &mut out)? };
    Ok(CaptureBytes::Emitted(out))
}

/// Orders the `(layout, value)*` capture bytes of two closures over targets
/// that already compared equal, so both captured `count` values.
///
/// Each side is read under its own embedded layouts. A closure loaded from
/// storage keeps the layouts it was written with, which a later upgrade may
/// have changed, so the two sides can disagree on field counts.
fn compare_captures(a: &[u8], b: &[u8], count: usize) -> Result<Ordering, RuntimeError> {
    compare_capture_lists(&decode_captures(a, count)?, &decode_captures(b, count)?)
}

/// Compares two capture lists of the same length pair by pair.
fn compare_capture_lists(
    a: &[(MoveTypeLayout, MoveValue)],
    b: &[(MoveTypeLayout, MoveValue)],
) -> Result<Ordering, RuntimeError> {
    for ((layout_a, a), (layout_b, b)) in a.iter().zip(b) {
        let ord = compare_move_values(layout_a, a, layout_b, b)?;
        if ord.is_ne() {
            return Ok(ord);
        }
    }
    Ok(Ordering::Equal)
}

/// Two captures sit at the same position under layouts of different kinds.
///
/// V1 reports `INTERNAL_TYPE_ERROR` for a value-kind mismatch and defines no
/// cross-kind order. MonoMove has no equivalent error, so it falls back rather
/// than inventing one.
//
// TODO(correctness): `drop_unchanged_writes` compares at session close, where
// `Unsupported` aborts the block instead of falling back. See its own
// `TODO(correctness)` on auditing the `equals` error paths.
fn kind_mismatch() -> RuntimeError {
    RuntimeError::Unsupported("captures compared under layouts of different kinds")
}

/// A decoded value does not have the shape of the layout it was decoded under.
fn layout_mismatch() -> RuntimeError {
    RuntimeError::InvariantViolation(RuntimeInvariantViolation::Unreachable(
        "a decoded capture does not match its layout".to_string(),
    ))
}

/// Compares two decoded values, each under its own layout, by V1's rules.
/// Integers compare by value within one width and signedness. Vectors compare
/// element-wise, then by length. Structs compare field-wise, then by field
/// count. Enums compare by variant tag, then by the variant's fields. Closures
/// compare by target, mask, then captures.
//
// TODO(metering): recursion depth here follows the value. Decoding already
// bounds it by move-core's value depth limit.
fn compare_move_values(
    layout_a: &MoveTypeLayout,
    a: &MoveValue,
    layout_b: &MoveTypeLayout,
    b: &MoveValue,
) -> Result<Ordering, RuntimeError> {
    use MoveValue as V;
    Ok(match (a, b) {
        (V::Bool(a), V::Bool(b)) => a.cmp(b),
        (V::U8(a), V::U8(b)) => a.cmp(b),
        (V::U16(a), V::U16(b)) => a.cmp(b),
        (V::U32(a), V::U32(b)) => a.cmp(b),
        (V::U64(a), V::U64(b)) => a.cmp(b),
        (V::U128(a), V::U128(b)) => a.cmp(b),
        (V::U256(a), V::U256(b)) => a.cmp(b),
        (V::I8(a), V::I8(b)) => a.cmp(b),
        (V::I16(a), V::I16(b)) => a.cmp(b),
        (V::I32(a), V::I32(b)) => a.cmp(b),
        (V::I64(a), V::I64(b)) => a.cmp(b),
        (V::I128(a), V::I128(b)) => a.cmp(b),
        (V::I256(a), V::I256(b)) => a.cmp(b),
        (V::Address(a), V::Address(b)) => a.cmp(b),
        (V::Vector(a), V::Vector(b)) => compare_move_vectors(layout_a, a, layout_b, b)?,
        (V::Struct(a), V::Struct(b)) => compare_move_structs(layout_a, a, layout_b, b)?,
        (V::Closure(a), V::Closure(b)) => compare_move_closures(a, b)?,
        // V1 holds each integer width in its own variant, so `u64` against
        // `u8` is a type error there, not a numeric comparison. Move-core
        // never decodes a `signer`.
        (
            V::Bool(_)
            | V::U8(_)
            | V::U16(_)
            | V::U32(_)
            | V::U64(_)
            | V::U128(_)
            | V::U256(_)
            | V::I8(_)
            | V::I16(_)
            | V::I32(_)
            | V::I64(_)
            | V::I128(_)
            | V::I256(_)
            | V::Address(_)
            | V::Vector(_)
            | V::Struct(_)
            | V::Signer(_)
            | V::Closure(_),
            _,
        ) => return Err(kind_mismatch()),
    })
}

fn compare_move_vectors(
    layout_a: &MoveTypeLayout,
    a: &[MoveValue],
    layout_b: &MoveTypeLayout,
    b: &[MoveValue],
) -> Result<Ordering, RuntimeError> {
    let (MoveTypeLayout::Vector(elem_a), MoveTypeLayout::Vector(elem_b)) = (layout_a, layout_b)
    else {
        return Err(layout_mismatch());
    };
    // V1 stores a vector of primitives in a container variant picked by the
    // element type and compares the variants before the elements, so two
    // vectors whose element kinds disagree are a type error there even when
    // both are empty or lengths already differ.
    if vector_class(elem_a) != vector_class(elem_b) {
        return Err(kind_mismatch());
    }
    for (a, b) in a.iter().zip(b) {
        let ord = compare_move_values(elem_a, a, elem_b, b)?;
        if ord.is_ne() {
            return Ok(ord);
        }
    }
    Ok(a.len().cmp(&b.len()))
}

/// The V1 container variant a vector of this element layout takes: a dedicated
/// one per primitive element type, and one shared by everything else.
fn vector_class(elem: &MoveTypeLayout) -> Option<std::mem::Discriminant<MoveTypeLayout>> {
    use MoveTypeLayout as L;
    match elem {
        L::Bool
        | L::U8
        | L::U16
        | L::U32
        | L::U64
        | L::U128
        | L::U256
        | L::I8
        | L::I16
        | L::I32
        | L::I64
        | L::I128
        | L::I256
        | L::Address => Some(std::mem::discriminant(elem)),
        L::Vector(_) | L::Struct(_) | L::Signer | L::Native(..) | L::Function => None,
    }
}

fn compare_move_structs(
    layout_a: &MoveTypeLayout,
    a: &MoveStruct,
    layout_b: &MoveTypeLayout,
    b: &MoveStruct,
) -> Result<Ordering, RuntimeError> {
    let (MoveTypeLayout::Struct(layout_a), MoveTypeLayout::Struct(layout_b)) = (layout_a, layout_b)
    else {
        return Err(layout_mismatch());
    };
    match (a, b) {
        (MoveStruct::Runtime(a), MoveStruct::Runtime(b)) => {
            let (MoveStructLayout::Runtime(fields_a), MoveStructLayout::Runtime(fields_b)) =
                (layout_a.as_ref(), layout_b.as_ref())
            else {
                return Err(layout_mismatch());
            };
            compare_move_fields(fields_a, a, fields_b, b)
        },
        (MoveStruct::RuntimeVariant(tag_a, a), MoveStruct::RuntimeVariant(tag_b, b)) => {
            // V1 decodes an enum into a struct whose first field is the variant
            // tag, so the tag orders ahead of the fields.
            if tag_a != tag_b {
                return Ok(tag_a.cmp(tag_b));
            }
            let (
                MoveStructLayout::RuntimeVariants(variants_a),
                MoveStructLayout::RuntimeVariants(variants_b),
            ) = (layout_a.as_ref(), layout_b.as_ref())
            else {
                return Err(layout_mismatch());
            };
            let (Some(fields_a), Some(fields_b)) = (
                variants_a.get(*tag_a as usize),
                variants_b.get(*tag_b as usize),
            ) else {
                return Err(layout_mismatch());
            };
            compare_move_fields(fields_a, a, fields_b, b)
        },
        // A struct against an enum is a type error in V1. V1 cannot decode
        // the decorated forms at all.
        (
            MoveStruct::Runtime(_)
            | MoveStruct::RuntimeVariant(..)
            | MoveStruct::WithFields(_)
            | MoveStruct::WithTypes { .. }
            | MoveStruct::WithVariantFields(..),
            _,
        ) => Err(kind_mismatch()),
    }
}

/// Compares two runs of fields value by value, then by count.
fn compare_move_fields(
    layouts_a: &[MoveTypeLayout],
    a: &[MoveValue],
    layouts_b: &[MoveTypeLayout],
    b: &[MoveValue],
) -> Result<Ordering, RuntimeError> {
    for ((layout_a, a), (layout_b, b)) in layouts_a.iter().zip(a).zip(layouts_b.iter().zip(b)) {
        let ord = compare_move_values(layout_a, a, layout_b, b)?;
        if ord.is_ne() {
            return Ok(ord);
        }
    }
    Ok(a.len().cmp(&b.len()))
}

/// Orders two nested closures by target, then mask, the order V1's `cmp_dyn`
/// defines, then by their captures.
fn compare_move_closures(a: &MoveClosure, b: &MoveClosure) -> Result<Ordering, RuntimeError> {
    let ord = a
        .module_id
        .address()
        .cmp(b.module_id.address())
        .then_with(|| a.module_id.name().cmp(b.module_id.name()))
        .then_with(|| a.fun_id.cmp(&b.fun_id))
        .then_with(|| a.ty_args.cmp(&b.ty_args))
        .then_with(|| a.mask.cmp(&b.mask));
    if ord.is_ne() {
        return Ok(ord);
    }
    // Equal masks mean equal capture counts.
    compare_capture_lists(&a.captured, &b.captured)
}

/// Orders two function references the way V1's `cmp_dyn` orders closure
/// targets: module id, then function name, then type arguments.
fn compare_func_refs(a: &FunctionRef, b: &FunctionRef) -> VMResult<Ordering> {
    let (module_a, module_b) = (view_module_id(a.module_id), view_module_id(b.module_id));
    let ord = module_a
        .address()
        .cmp(module_b.address())
        .then_with(|| view_name(module_a.name()).cmp(view_name(module_b.name())))
        .then_with(|| view_name(a.func_name).cmp(view_name(b.func_name)));
    if ord.is_ne() {
        return Ok(ord);
    }
    Ok(func_ref_ty_args(a)?.cmp(&func_ref_ty_args(b)?))
}

fn func_ref_ty_args(func_ref: &FunctionRef) -> VMResult<Vec<TypeTag>> {
    view_type_list(func_ref.ty_args)
        .iter()
        .map(|&ty| {
            type_tag_of(ty).ok_or_else(|| {
                VMInternalError::new(RuntimeError::Unsupported(
                    "function value over a type with no type tag",
                ))
            })
        })
        .collect::<VMResult<Vec<TypeTag>>>()
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

#[cfg(test)]
mod tests {
    use super::*;
    use move_core_types::{
        account_address::AccountAddress,
        function::{ClosureMask, FUNCTION_DATA_SERIALIZATION_FORMAT_V1},
        identifier::Identifier,
        language_storage::ModuleId,
        value::IdentifierMappingKind,
    };
    use serde::Serialize;
    use std::sync::Arc;

    /// Appends one `(layout, value)` capture pair.
    fn capture<V: Serialize>(out: &mut Vec<u8>, layout: &MoveTypeLayout, value: &V) {
        out.extend_from_slice(&bcs::to_bytes(layout).unwrap());
        out.extend_from_slice(&bcs::to_bytes(value).unwrap());
    }

    /// Capture bytes holding a single `(layout, value)` pair.
    fn one_capture<V: Serialize>(layout: &MoveTypeLayout, value: &V) -> Vec<u8> {
        let mut out = vec![];
        capture(&mut out, layout, value);
        out
    }

    /// The wire bytes of a closure over `0x1::m::f`, followed by `captures`,
    /// the already encoded capture pairs.
    fn closure_bytes(ty_args: &[TypeTag], mask: u64, captures: &[u8]) -> Vec<u8> {
        // The `5 + 2n` sequence length fits one ULEB128 byte.
        let mut out = vec![5 + 2 * mask.count_ones() as u8];
        out.extend(bcs::to_bytes(&FUNCTION_DATA_SERIALIZATION_FORMAT_V1).unwrap());
        let module = ModuleId::new(AccountAddress::ONE, Identifier::new("m").unwrap());
        out.extend(bcs::to_bytes(&module).unwrap());
        out.extend(bcs::to_bytes(&Identifier::new("f").unwrap()).unwrap());
        out.extend(bcs::to_bytes(&ty_args.to_vec()).unwrap());
        out.extend(bcs::to_bytes(&ClosureMask::new(mask)).unwrap());
        out.extend_from_slice(captures);
        out
    }

    fn cmp_one<V: Serialize, W: Serialize>(
        layout_a: &MoveTypeLayout,
        a: &V,
        layout_b: &MoveTypeLayout,
        b: &W,
    ) -> Result<Ordering, RuntimeError> {
        compare_captures(&one_capture(layout_a, a), &one_capture(layout_b, b), 1)
    }

    /// Compares `a` against `b` under one shared layout.
    fn cmp_same<V: Serialize, W: Serialize>(
        layout: &MoveTypeLayout,
        a: &V,
        b: &W,
    ) -> Result<Ordering, RuntimeError> {
        cmp_one(layout, a, layout, b)
    }

    #[test]
    fn compare_unsigned_is_numeric_not_byte_order() {
        // Little-endian bytes order these the other way round.
        assert_eq!(
            cmp_same(&MoveTypeLayout::U64, &1u64, &256u64).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            cmp_same(&MoveTypeLayout::U128, &u128::MAX, &0u128).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            cmp_same(&MoveTypeLayout::U16, &7u16, &7u16).unwrap(),
            Ordering::Equal
        );
    }

    #[test]
    fn compare_signed_orders_by_sign_first() {
        for (a, b, expected) in [
            (-1i64, 0i64, Ordering::Less),
            (0, -1, Ordering::Greater),
            (-2, -1, Ordering::Less),
            (i64::MIN, i64::MAX, Ordering::Less),
            (-5, -5, Ordering::Equal),
        ] {
            assert_eq!(
                cmp_same(&MoveTypeLayout::I64, &a, &b).unwrap(),
                expected,
                "{a} against {b}"
            );
        }
    }

    #[test]
    fn compare_bools_and_addresses() {
        assert_eq!(
            cmp_same(&MoveTypeLayout::Bool, &false, &true).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            cmp_same(
                &MoveTypeLayout::Address,
                &AccountAddress::ONE,
                &AccountAddress::TWO
            )
            .unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn compare_vectors_by_element_then_length() {
        let layout = MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8));
        // The longer vector loses on its first element, which the ULEB length
        // prefix would hide from a byte comparison.
        assert_eq!(
            cmp_same(&layout, &vec![9u8], &vec![1u8, 1]).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            cmp_same(&layout, &vec![1u8], &vec![1u8, 1]).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            cmp_same(&layout, &Vec::<u8>::new(), &Vec::<u8>::new()).unwrap(),
            Ordering::Equal
        );
    }

    #[test]
    fn compare_vectors_of_different_element_kinds_is_a_type_error() {
        // V1 puts these in different container variants and compares the
        // variants before it looks inside, so even two empty ones are an error.
        let u8s = MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8));
        let u64s = MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U64));
        for (a, b) in [(vec![], vec![]), (vec![1u8], vec![1u64])] {
            assert!(matches!(
                cmp_one(&u8s, &a, &u64s, &b).unwrap_err(),
                RuntimeError::Unsupported(_)
            ));
        }
    }

    #[test]
    fn compare_struct_by_field_then_count() {
        let one = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
        ])));
        let two = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
            MoveTypeLayout::U64,
        ])));
        // A stored closure can carry a pre-upgrade layout, so differing field
        // counts order by count rather than erroring.
        assert_eq!(
            cmp_one(&one, &(7u64,), &two, &(7u64, 0u64)).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            cmp_one(&one, &(8u64,), &two, &(7u64, 0u64)).unwrap(),
            Ordering::Greater
        );
        assert_eq!(
            cmp_same(&two, &(7u64, 1u64), &(7u64, 1u64)).unwrap(),
            Ordering::Equal
        );
    }

    #[test]
    fn compare_struct_against_primitive_is_a_type_error() {
        let s = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
        ])));
        assert!(matches!(
            cmp_one(&s, &(7u64,), &MoveTypeLayout::U64, &7u64).unwrap_err(),
            RuntimeError::Unsupported(_)
        ));
    }

    #[test]
    fn compare_ints_of_different_widths_is_a_type_error() {
        assert!(matches!(
            cmp_one(&MoveTypeLayout::U8, &0u8, &MoveTypeLayout::U64, &0u64).unwrap_err(),
            RuntimeError::Unsupported(_)
        ));
        assert!(matches!(
            cmp_one(&MoveTypeLayout::U64, &0u64, &MoveTypeLayout::I64, &0i64).unwrap_err(),
            RuntimeError::Unsupported(_)
        ));
    }

    /// Three variants of two, zero and one `u64` field.
    fn enum_layout() -> MoveTypeLayout {
        MoveTypeLayout::Struct(Arc::new(MoveStructLayout::RuntimeVariants(vec![
            vec![MoveTypeLayout::U64, MoveTypeLayout::U64],
            vec![],
            vec![MoveTypeLayout::U64],
        ])))
    }

    /// Capture bytes holding one value of [`enum_layout`].
    fn enum_capture(tag: u8, fields: &[u64]) -> Vec<u8> {
        let mut out = bcs::to_bytes(&enum_layout()).unwrap();
        // The variant tag fits one ULEB128 byte.
        out.push(tag);
        for value in fields {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    #[test]
    fn compare_enum_by_tag_then_fields() {
        let cases = [
            ((0u8, vec![1u64, 2]), (1u8, vec![]), Ordering::Less),
            ((2, vec![3]), (1, vec![]), Ordering::Greater),
            ((0, vec![1, 2]), (0, vec![1, 3]), Ordering::Less),
            ((2, vec![3]), (2, vec![3]), Ordering::Equal),
        ];
        for ((tag_a, fields_a), (tag_b, fields_b), expected) in cases {
            let ord = compare_captures(
                &enum_capture(tag_a, &fields_a),
                &enum_capture(tag_b, &fields_b),
                1,
            )
            .unwrap();
            assert_eq!(ord, expected, "variant {tag_a} against {tag_b}");
        }
    }

    #[test]
    fn compare_reaches_the_capture_after_an_enum() {
        // The first capture must decode exactly, whichever variant it selects,
        // for the second one to be compared at all.
        for (tag, fields) in [(0u8, vec![1u64, 2]), (1, vec![]), (2, vec![3])] {
            let mut bytes = enum_capture(tag, &fields);
            capture(&mut bytes, &MoveTypeLayout::U64, &9u64);
            assert_eq!(
                compare_captures(&bytes, &bytes, 2).unwrap(),
                Ordering::Equal
            );

            let mut other = enum_capture(tag, &fields);
            capture(&mut other, &MoveTypeLayout::U64, &10u64);
            assert_eq!(compare_captures(&bytes, &other, 2).unwrap(), Ordering::Less);
        }
    }

    #[test]
    fn compare_reaches_the_capture_after_a_vector() {
        for elements in [vec![], vec![1u64], vec![1u64, 2]] {
            let mut bytes = one_capture(
                &MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U64)),
                &elements,
            );
            capture(&mut bytes, &MoveTypeLayout::U64, &9u64);
            let mut other = bytes.clone();
            *other.last_mut().unwrap() = 1;
            assert_eq!(
                compare_captures(&bytes, &bytes, 2).unwrap(),
                Ordering::Equal
            );
            assert_eq!(
                compare_captures(&bytes, &other, 2).unwrap(),
                Ordering::Less,
                "{elements:?}"
            );
        }
    }

    #[test]
    fn compare_nested_closures() {
        let inner = |value: u64| {
            let mut out = bcs::to_bytes(&MoveTypeLayout::Function).unwrap();
            out.extend_from_slice(&closure_bytes(
                &[],
                0b1,
                &one_capture(&MoveTypeLayout::U64, &value),
            ));
            out
        };
        assert_eq!(
            compare_captures(&inner(1), &inner(1), 1).unwrap(),
            Ordering::Equal
        );
        assert_eq!(
            compare_captures(&inner(1), &inner(2), 1).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn compare_nested_closures_by_target() {
        // Same shape, different type arguments: the header decides.
        let closure = |ty_args: &[TypeTag]| {
            let mut out = bcs::to_bytes(&MoveTypeLayout::Function).unwrap();
            out.extend_from_slice(&closure_bytes(ty_args, 0, &[]));
            out
        };
        assert_eq!(
            compare_captures(&closure(&[TypeTag::U8]), &closure(&[TypeTag::U64]), 1).unwrap(),
            Ordering::Less
        );
        assert_eq!(
            compare_captures(&closure(&[]), &closure(&[TypeTag::U8]), 1).unwrap(),
            Ordering::Less
        );
    }

    #[test]
    fn compare_is_a_total_order() {
        // Sorting by the comparison must agree with it pairwise.
        let layout = MoveTypeLayout::Struct(Arc::new(MoveStructLayout::Runtime(vec![
            MoveTypeLayout::U64,
            MoveTypeLayout::Vector(Box::new(MoveTypeLayout::U8)),
        ])));
        let values = [
            (0u64, vec![]),
            (0u64, vec![1u8]),
            (1u64, vec![]),
            (1u64, vec![0u8, 0]),
            (u64::MAX, vec![255u8]),
        ];
        for (i, a) in values.iter().enumerate() {
            for (j, b) in values.iter().enumerate() {
                assert_eq!(
                    cmp_same(&layout, a, b).unwrap(),
                    i.cmp(&j),
                    "{a:?} against {b:?}"
                );
            }
        }
    }

    #[test]
    fn compare_rejects_signer_and_native() {
        // Move-core decodes neither, so a capture holding one cannot be read.
        for layout in [
            MoveTypeLayout::Signer,
            MoveTypeLayout::Native(
                IdentifierMappingKind::Aggregator,
                Box::new(MoveTypeLayout::U64),
            ),
        ] {
            let bytes = one_capture(&layout, &AccountAddress::ONE);
            assert!(matches!(
                compare_captures(&bytes, &bytes, 1).unwrap_err(),
                RuntimeError::BCSInvalidClosure("captured value")
            ));
        }
    }

    #[test]
    fn compare_rejects_truncation() {
        let bytes = one_capture(&MoveTypeLayout::U64, &7u64);
        for end in 1..bytes.len() {
            assert!(
                compare_captures(&bytes[..end], &bytes, 1).is_err(),
                "truncating to {end} bytes must fail"
            );
        }
    }
}
