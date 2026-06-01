// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Type-driven structural comparison of values.
//!
//! [`structural_compare`] is the read-only analog of the heap's deep copy: it
//! walks two values of the same type in lockstep and produces a 3-way
//! [`Ordering`]. Because the VM is monomorphized, the operand type is known
//! statically at the comparison site and threads down the recursion — nothing
//! is read from object headers. `Eq`/`Neq` derive from the result (`Equal` vs
//! not).
//!
//! Ordering matches the production Move VM's structural comparison:
//!   - Primitives compare numerically (`bool`: `false < true`; `address`: by
//!     byte order).
//!   - Structs compare field-by-field in declaration order; the first
//!     non-`Equal` field decides.
//!   - Vectors compare element-by-element; on a common prefix, the shorter
//!     vector is `Less`.
//!   - References are compared by their referent.
//!
//! Enums, function values, and signer are not supported yet and surface a
//! [`RuntimeInvariantViolation::UnsupportedComparisonType`].
//!
//! TODO(security): this is recursive; nested vectors make the depth
//! data-dependent. Add a depth bound (as planned for `try_deep_copy`).

use crate::{
    error::{RuntimeError, RuntimeInvariantViolation, RuntimeResult},
    interpreter::read_int,
    memory::{read_fat_ptr, read_ptr, read_u64},
    types::{VEC_DATA_OFFSET, VEC_LENGTH_OFFSET},
};
use mono_move_core::types::{view_type, InternedType, Type};
use move_core_types::{
    account_address::AccountAddress,
    int256::{I256, U256},
};
use std::cmp::Ordering;

/// Compare two values of type `ty`, returning their structural [`Ordering`].
///
/// # Safety
///
/// `lhs` and `rhs` must each point to the start of a fully-initialized value
/// of type `ty`, laid out per the runtime's value representation (inline for
/// primitives and structs, an 8-byte heap pointer for vectors, a 16-byte fat
/// pointer for references). For references the referent must be live.
pub(crate) unsafe fn structural_compare(
    ty: InternedType,
    lhs: *const u8,
    rhs: *const u8,
) -> RuntimeResult<Ordering> {
    // SAFETY: caller's contract — `lhs`/`rhs` point at values of type `ty`.
    unsafe {
        match view_type(ty) {
            Type::Bool => Ok(read_int::<u8>(lhs, 0usize).cmp(&read_int::<u8>(rhs, 0usize))),
            Type::U8 => Ok(read_int::<u8>(lhs, 0usize).cmp(&read_int::<u8>(rhs, 0usize))),
            Type::U16 => Ok(read_int::<u16>(lhs, 0usize).cmp(&read_int::<u16>(rhs, 0usize))),
            Type::U32 => Ok(read_int::<u32>(lhs, 0usize).cmp(&read_int::<u32>(rhs, 0usize))),
            Type::U64 => Ok(read_int::<u64>(lhs, 0usize).cmp(&read_int::<u64>(rhs, 0usize))),
            Type::U128 => Ok(read_int::<u128>(lhs, 0usize).cmp(&read_int::<u128>(rhs, 0usize))),
            Type::U256 => Ok(read_int::<U256>(lhs, 0usize).cmp(&read_int::<U256>(rhs, 0usize))),
            Type::I8 => Ok(read_int::<i8>(lhs, 0usize).cmp(&read_int::<i8>(rhs, 0usize))),
            Type::I16 => Ok(read_int::<i16>(lhs, 0usize).cmp(&read_int::<i16>(rhs, 0usize))),
            Type::I32 => Ok(read_int::<i32>(lhs, 0usize).cmp(&read_int::<i32>(rhs, 0usize))),
            Type::I64 => Ok(read_int::<i64>(lhs, 0usize).cmp(&read_int::<i64>(rhs, 0usize))),
            Type::I128 => Ok(read_int::<i128>(lhs, 0usize).cmp(&read_int::<i128>(rhs, 0usize))),
            Type::I256 => Ok(read_int::<I256>(lhs, 0usize).cmp(&read_int::<I256>(rhs, 0usize))),
            Type::Address => {
                Ok(read_int::<AccountAddress>(lhs, 0usize)
                    .cmp(&read_int::<AccountAddress>(rhs, 0usize)))
            },
            Type::Vector { elem } => compare_vector(*elem, lhs, rhs),
            Type::Nominal { .. } => compare_nominal(ty, lhs, rhs),
            Type::ImmutRef { inner } | Type::MutRef { inner } => {
                // Each side is a 16-byte fat pointer `(base, offset)`; the
                // referent lives at `base + offset`.
                let (l_base, l_off) = read_fat_ptr(lhs, 0usize);
                let (r_base, r_off) = read_fat_ptr(rhs, 0usize);
                structural_compare(
                    *inner,
                    l_base.add(l_off as usize),
                    r_base.add(r_off as usize),
                )
            },
            Type::Signer => Err(unsupported("signer")),
            Type::Function { .. } => Err(unsupported("function")),
            Type::TypeParam { .. } => Err(unsupported("type parameter")),
        }
    }
}

/// Compare two inline struct values field-by-field in declaration order.
///
/// # Safety
///
/// `lhs`/`rhs` point at inline struct values of nominal type `ty`.
unsafe fn compare_nominal(
    ty: InternedType,
    lhs: *const u8,
    rhs: *const u8,
) -> RuntimeResult<Ordering> {
    let layout = view_type(ty)
        .layout()
        .ok_or_else(|| unsupported("nominal type without populated layout"))?;
    // `field_layouts()` is `Some` for structs and `None` for enums; enum
    // comparison is not supported yet.
    let fields = layout.field_layouts().ok_or_else(|| unsupported("enum"))?;
    for field in fields {
        let off = field.offset as usize;
        // SAFETY: the field offset is in-bounds of the struct's inline payload.
        let ord = unsafe { structural_compare(field.ty(), lhs.add(off), rhs.add(off))? };
        if ord != Ordering::Equal {
            return Ok(ord);
        }
    }
    Ok(Ordering::Equal)
}

/// Compare two vectors lexicographically: element-by-element, then by length.
///
/// # Safety
///
/// `lhs`/`rhs` point at the 8-byte vector pointer slots (null = empty).
unsafe fn compare_vector(
    elem_ty: InternedType,
    lhs: *const u8,
    rhs: *const u8,
) -> RuntimeResult<Ordering> {
    // SAFETY: caller's contract — each side is an 8-byte vector pointer slot.
    let l_vec = unsafe { read_ptr(lhs, 0usize) };
    let r_vec = unsafe { read_ptr(rhs, 0usize) };
    let l_len = if l_vec.is_null() {
        0
    } else {
        unsafe { read_u64(l_vec, VEC_LENGTH_OFFSET) }
    };
    let r_len = if r_vec.is_null() {
        0
    } else {
        unsafe { read_u64(r_vec, VEC_LENGTH_OFFSET) }
    };

    let (elem_size, _) = view_type(elem_ty)
        .size_and_align()
        .ok_or_else(|| unsupported("vector element without populated layout"))?;
    let elem_size = elem_size as usize;

    let common = l_len.min(r_len);
    for i in 0..common {
        let off = VEC_DATA_OFFSET + (i as usize) * elem_size;
        // SAFETY: `i < len`, so the element at `off` is within the data region.
        let ord = unsafe { structural_compare(elem_ty, l_vec.add(off), r_vec.add(off))? };
        if ord != Ordering::Equal {
            return Ok(ord);
        }
    }
    Ok(l_len.cmp(&r_len))
}

fn unsupported(kind: &'static str) -> RuntimeError {
    RuntimeError::InvariantViolation(RuntimeInvariantViolation::UnsupportedComparisonType { kind })
}
