// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type-driven BCS serialization of a runtime value.
//!
//! This is the type-driven sibling of `deep_copy` (`crate::heap::deep_copy`).
//! Both walk the value tree of a Move value, but `deep_copy` is
//! descriptor-driven and only needs to clone raw bytes and patch child
//! pointers, whereas serialization must know each value's type: BCS encoding
//! differs from the in-memory layout (vectors carry a ULEB128 length prefix
//! instead of the stored length, struct padding bytes are dropped, and
//! primitive widths decide how many bytes to emit). The type carries
//! everything needed — per-field offsets on `NominalLayout`, the vector element
//! type, and `size_and_align` — so this code never reads object headers,
//! allocates on the VM heap, or touches the GC. It only reads through pointers
//! and appends to an output buffer.
//!
//! Layout assumption: a struct value is laid out inline (its primitives and any
//! nested structs sit flat in its data region at their field offsets), while a
//! vector slot holds an 8-byte heap pointer (null for an empty vector). This
//! matches the layout `deep_copy` and `NominalLayout` agree on.
//!
//! Out of scope (returns [`ValueSerializationError::UnsupportedType`]): enums,
//! function/closure values, references, signed integers, and type parameters.
//!
//! TODO(security): the walk is recursive with a depth limit. Revisit with a
//! non-recursive algorithm if needed, mirroring the note on `deep_copy`.

use crate::{
    error::{RuntimeResult, ValueSerializationError},
    memory::{read_ptr, read_u64},
    types::{VEC_DATA_OFFSET, VEC_LENGTH_OFFSET},
};
use mono_move_core::types::{view_type, InternedType, Type};

/// Maximum value-tree depth the serializer will descend before bailing.
const MAX_SERIALIZE_DEPTH: usize = 256;

/// Maximum length of a BCS sequence. Mirrors `bcs::MAX_SEQUENCE_LENGTH`, which
/// is not depended on directly to keep `bcs` out of the runtime's build graph.
const MAX_SEQUENCE_LENGTH: u64 = (1 << 31) - 1;

/// Serializes the value of type `ty` stored at `base` into its BCS bytes.
///
/// # Safety
///
/// `base` must point to a fully initialized value of type `ty`, laid out per
/// the runtime value representation (primitives and inline structs flat, vector
/// slots holding heap pointers), and must remain valid for the call. Any heap
/// objects reachable from the value must be live.
pub unsafe fn serialize_value(base: *const u8, ty: InternedType) -> RuntimeResult<Vec<u8>> {
    let mut out = vec![];
    // SAFETY: forwarded from this function's contract.
    unsafe { serialize_into(&mut out, base, ty, 0)? };
    Ok(out)
}

/// Returns the number of bytes `serialize_value` would produce for the value of
/// type `ty` stored at `base`.
///
/// # Safety
///
/// Same contract as [`serialize_value`].
pub unsafe fn serialized_value_size(base: *const u8, ty: InternedType) -> RuntimeResult<usize> {
    // SAFETY: forwarded from this function's contract.
    Ok(unsafe { serialize_value(base, ty)? }.len())
}

/// Recursive worker: appends the BCS encoding of the value at `base` to `out`.
///
/// # Safety
///
/// Same contract as [`serialize_value`], for the sub-value at `base`.
unsafe fn serialize_into(
    out: &mut Vec<u8>,
    base: *const u8,
    ty: InternedType,
    depth: usize,
) -> RuntimeResult<()> {
    if depth > MAX_SERIALIZE_DEPTH {
        return Err(ValueSerializationError::NestingTooDeep.into());
    }

    let view = view_type(ty);
    match view {
        // Primitives: the in-memory little-endian bytes are already the BCS
        // encoding. `bool` is a single byte the VM guarantees is canonical (0
        // or 1), matching BCS; `address`/`signer` are 32 bytes. The byte count
        // comes straight from the type.
        Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::Address
        | Type::Signer => {
            let (size, _) = view
                .size_and_align()
                .ok_or(ValueSerializationError::LayoutUnavailable)?;
            // SAFETY: caller guarantees `base..base + size` is an initialized
            // value of this primitive type.
            unsafe { append_bytes(out, base, size as usize) };
        },

        Type::Vector { elem } => {
            // A vector slot is an 8-byte heap pointer; null means empty.
            // SAFETY: caller guarantees `base` holds a vector pointer.
            let ptr = unsafe { read_ptr(base, 0usize) };
            let len = if ptr.is_null() {
                0
            } else {
                // SAFETY: the length is a `u64` at the start of the data region
                // of a live vector object.
                unsafe { read_u64(ptr, VEC_LENGTH_OFFSET) }
            };
            if len > MAX_SEQUENCE_LENGTH {
                return Err(ValueSerializationError::SequenceTooLong {
                    len,
                    max: MAX_SEQUENCE_LENGTH,
                }
                .into());
            }
            write_uleb128(out, len);
            // An empty sequence is just its length prefix, for any element
            // type — so we never dereference a null pointer or require the
            // element layout.
            if len == 0 {
                return Ok(());
            }

            let elem_view = view_type(*elem);
            let (elem_size, _) = elem_view
                .size_and_align()
                .ok_or(ValueSerializationError::LayoutUnavailable)?;
            let elem_size = elem_size as usize;
            // SAFETY: the elements start at `VEC_DATA_OFFSET` in a live vector.
            let data = unsafe { ptr.add(VEC_DATA_OFFSET) };

            if is_scalar_primitive(elem_view) {
                // Scalar elements are fixed-width with no padding or inner
                // pointers, so their contiguous little-endian bytes are exactly
                // the BCS encoding of the sequence body. Copy them in one shot.
                // SAFETY: `data..data + len * elem_size` is the element region
                // of a live vector of length `len`.
                unsafe { append_bytes(out, data, len as usize * elem_size) };
            } else {
                for i in 0..len as usize {
                    // SAFETY: element `i` lives at `data + i * elem_size`, in
                    // bounds for a vector of length `len`.
                    let elem_ptr = unsafe { data.add(i * elem_size) };
                    // SAFETY: `elem_ptr` is an initialized value of type `*elem`.
                    unsafe { serialize_into(out, elem_ptr, *elem, depth + 1)? };
                }
            }
        },

        Type::Nominal { layout, .. } => {
            let layout = layout
                .get()
                .ok_or(ValueSerializationError::LayoutUnavailable)?;
            match layout.field_layouts() {
                // Struct: fields are laid out inline at their offsets and BCS
                // is their encodings concatenated in declaration order.
                Some(fields) => {
                    for field in fields {
                        // SAFETY: the field lives at `base + offset` within the
                        // inline struct data, per its computed layout.
                        let field_ptr = unsafe { base.add(field.offset as usize) };
                        // SAFETY: `field_ptr` is an initialized value of the
                        // field's type.
                        unsafe { serialize_into(out, field_ptr, field.ty(), depth + 1)? };
                    }
                },
                // Enum (no per-field offsets): not supported yet.
                None => return Err(ValueSerializationError::UnsupportedType.into()),
            }
        },

        // References, function values, signed integers, and unresolved type
        // parameters are not BCS-serializable.
        Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Function { .. }
        | Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256
        | Type::TypeParam { .. } => return Err(ValueSerializationError::UnsupportedType.into()),
    }
    Ok(())
}

/// Whether `ty` is a fixed-width scalar whose in-memory little-endian bytes are
/// exactly its BCS encoding — no padding and no inner pointers. Vectors of such
/// elements can be bulk-copied instead of recursing per element. Structs are
/// excluded because they may carry inter-field padding, and signed integers /
/// references / functions / vectors / type parameters are not handled here.
fn is_scalar_primitive(ty: &Type) -> bool {
    match ty {
        Type::Bool
        | Type::U8
        | Type::U16
        | Type::U32
        | Type::U64
        | Type::U128
        | Type::U256
        | Type::Address
        | Type::Signer => true,
        Type::I8
        | Type::I16
        | Type::I32
        | Type::I64
        | Type::I128
        | Type::I256
        | Type::Vector { .. }
        | Type::Nominal { .. }
        | Type::ImmutRef { .. }
        | Type::MutRef { .. }
        | Type::Function { .. }
        | Type::TypeParam { .. } => false,
    }
}

/// Appends `len` bytes read from `base` to `out`.
///
/// # Safety
///
/// `base..base + len` must be valid for reads of initialized bytes.
#[inline]
unsafe fn append_bytes(out: &mut Vec<u8>, base: *const u8, len: usize) {
    let start = out.len();
    out.reserve(len);
    // SAFETY: `reserve` guarantees `len` writable bytes at `start`; the source
    // range is valid per this function's contract; the two regions do not
    // overlap (the output buffer is a fresh allocation).
    unsafe {
        std::ptr::copy_nonoverlapping(base, out.as_mut_ptr().add(start), len);
        out.set_len(start + len);
    }
}

/// Writes `value` as ULEB128, matching BCS's length encoding.
fn write_uleb128(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
}
