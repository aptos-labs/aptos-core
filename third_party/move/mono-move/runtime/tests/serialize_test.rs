// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for type-driven BCS serialization of runtime values.
//!
//! Each case fabricates the value's in-memory representation (primitives flat,
//! inline structs flat, vectors as heap pointers), serializes it, and compares
//! against the `bcs` crate as the oracle. Memory is built with `MemoryRegion`,
//! which is `MAX_ALIGN`-aligned and zeroed, so wide reads stay aligned.

use mono_move_core::{
    types::{FieldLayout, ADDRESS_TY, BOOL_TY, EMPTY_TYPE_LIST, U128_TY, U256_TY, U64_TY, U8_TY},
    Interner,
};
use mono_move_global_context::GlobalContext;
use mono_move_runtime::{
    serialize_value, serialized_value_size, write_ptr, write_u64, MemoryRegion, RuntimeError,
    ValueSerializationError, VEC_DATA_OFFSET,
};
use move_core_types::{account_address::AccountAddress, ident_str};
use serde::Serialize;

/// A `MemoryRegion` holding `bytes` at offset 0.
fn region_from_bytes(bytes: &[u8]) -> MemoryRegion {
    let region = MemoryRegion::new(bytes.len().max(1));
    // SAFETY: the region is at least `bytes.len()` bytes.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), region.as_ptr(), bytes.len()) };
    region
}

/// Builds a vector's heap data region: `[length: u64 | element bytes...]`.
fn vec_region(len: u64, elem_bytes: &[u8]) -> MemoryRegion {
    let region = MemoryRegion::new((VEC_DATA_OFFSET + elem_bytes.len()).max(8));
    // SAFETY: the region is large enough for the length and the elements.
    unsafe {
        write_u64(region.as_ptr(), 0usize, len);
        std::ptr::copy_nonoverlapping(
            elem_bytes.as_ptr(),
            region.as_ptr().add(VEC_DATA_OFFSET),
            elem_bytes.len(),
        );
    }
    region
}

/// An 8-byte slot holding `target` as a heap pointer.
fn ptr_slot(target: *const u8) -> MemoryRegion {
    let region = MemoryRegion::new(8);
    // SAFETY: the region is 8 bytes, exactly one pointer.
    unsafe { write_ptr(region.as_ptr(), 0usize, target) };
    region
}

/// Concatenated little-endian bytes of the given `u64`s.
fn u64_bytes(values: &[u64]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

#[test]
fn primitives_match_bcs() {
    let u64_val: u64 = 0x1122_3344_5566_7788;
    let region = region_from_bytes(&u64_val.to_le_bytes());
    let bytes = unsafe { serialize_value(region.as_ptr(), U64_TY) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&u64_val).unwrap());
    let size = unsafe { serialized_value_size(region.as_ptr(), U64_TY) }.unwrap();
    assert_eq!(size, bytes.len());

    let u8_val: u8 = 0xAB;
    let region = region_from_bytes(&[u8_val]);
    let bytes = unsafe { serialize_value(region.as_ptr(), U8_TY) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&u8_val).unwrap());

    let u128_val: u128 = 0x0102_0304_0506_0708_090A_0B0C_0D0E_0F10;
    let region = region_from_bytes(&u128_val.to_le_bytes());
    let bytes = unsafe { serialize_value(region.as_ptr(), U128_TY) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&u128_val).unwrap());

    // u256: BCS of a 256-bit integer is exactly its 32 little-endian bytes,
    // which is how the VM stores it, so the output equals the raw bytes.
    let mut u256_bytes = [0u8; 32];
    (0..32).for_each(|i| u256_bytes[i] = (i as u8).wrapping_mul(7));
    let region = region_from_bytes(&u256_bytes);
    let bytes = unsafe { serialize_value(region.as_ptr(), U256_TY) }.unwrap();
    assert_eq!(bytes, u256_bytes.to_vec());

    let region = region_from_bytes(&[1u8]);
    let bytes = unsafe { serialize_value(region.as_ptr(), BOOL_TY) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&true).unwrap());

    let mut addr_bytes = [0u8; 32];
    (0..32).for_each(|i| addr_bytes[i] = i as u8);
    let addr = AccountAddress::new(addr_bytes);
    let region = region_from_bytes(&addr_bytes);
    let bytes = unsafe { serialize_value(region.as_ptr(), ADDRESS_TY) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&addr).unwrap());
}

#[test]
fn vector_of_u64_matches_bcs() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let vec_ty = guard.vector_of(U64_TY);

    let values = vec![1u64, 2, 3, 0xDEAD_BEEF];
    let data = vec_region(values.len() as u64, &u64_bytes(&values));
    let slot = ptr_slot(data.as_ptr());

    let bytes = unsafe { serialize_value(slot.as_ptr(), vec_ty) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&values).unwrap());
    let size = unsafe { serialized_value_size(slot.as_ptr(), vec_ty) }.unwrap();
    assert_eq!(size, bytes.len());
}

#[test]
fn vector_of_u8_matches_bcs() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let vec_ty = guard.vector_of(U8_TY);

    let values: Vec<u8> = vec![10, 20, 30, 40, 50];
    let data = vec_region(values.len() as u64, &values);
    let slot = ptr_slot(data.as_ptr());

    let bytes = unsafe { serialize_value(slot.as_ptr(), vec_ty) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&values).unwrap());
}

#[test]
fn empty_vector_is_null_pointer() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let vec_ty = guard.vector_of(U64_TY);

    // A zeroed slot is a null pointer, i.e. an empty vector.
    let slot = MemoryRegion::new(8);
    let bytes = unsafe { serialize_value(slot.as_ptr(), vec_ty) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&Vec::<u64>::new()).unwrap());
}

#[test]
fn struct_with_primitive_and_vector_fields() {
    #[derive(Serialize)]
    struct S {
        a: u64,
        b: bool,
        c: Vec<u64>,
    }

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();

    let module_id = guard.module_id_of(&AccountAddress::ONE, ident_str!("m"));
    let s_ty = guard.intern_nominal(
        module_id,
        guard.identifier_of(ident_str!("S")),
        EMPTY_TYPE_LIST,
    );
    let vec_ty = guard.vector_of(U64_TY);

    // a: u64 @0, b: bool @8, c: vector<u64> @16 (8-aligned). Size 24.
    let fields = [
        FieldLayout::new(0, U64_TY),
        FieldLayout::new(8, BOOL_TY),
        FieldLayout::new(16, vec_ty),
    ];
    guard
        .set_nominal_layout(s_ty, 24, 8, Some(&fields))
        .unwrap();

    let vec_data = vec_region(2, &u64_bytes(&[7, 9]));
    let region = MemoryRegion::new(24);
    // SAFETY: the region is 24 bytes, matching the struct layout.
    unsafe {
        write_u64(region.as_ptr(), 0usize, 42);
        *region.as_ptr().add(8) = 1; // bool true
        write_ptr(region.as_ptr(), 16usize, vec_data.as_ptr());
    }

    let bytes = unsafe { serialize_value(region.as_ptr(), s_ty) }.unwrap();
    let oracle = S {
        a: 42,
        b: true,
        c: vec![7, 9],
    };
    assert_eq!(bytes, bcs::to_bytes(&oracle).unwrap());
}

#[test]
fn nested_struct_is_inline() {
    #[derive(Serialize)]
    struct Inner {
        x: u64,
        y: u64,
    }
    #[derive(Serialize)]
    struct Outer {
        a: u8,
        inner: Inner,
    }

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let module_id = guard.module_id_of(&AccountAddress::ONE, ident_str!("m"));

    // Inner { x: u64 @0, y: u64 @8 }, size 16.
    let inner_ty = guard.intern_nominal(
        module_id,
        guard.identifier_of(ident_str!("Inner")),
        EMPTY_TYPE_LIST,
    );
    let inner_fields = [FieldLayout::new(0, U64_TY), FieldLayout::new(8, U64_TY)];
    guard
        .set_nominal_layout(inner_ty, 16, 8, Some(&inner_fields))
        .unwrap();

    // Outer { a: u8 @0, inner: Inner @8 (8-aligned) }, size 24.
    let outer_ty = guard.intern_nominal(
        module_id,
        guard.identifier_of(ident_str!("Outer")),
        EMPTY_TYPE_LIST,
    );
    let outer_fields = [FieldLayout::new(0, U8_TY), FieldLayout::new(8, inner_ty)];
    guard
        .set_nominal_layout(outer_ty, 24, 8, Some(&outer_fields))
        .unwrap();

    let region = MemoryRegion::new(24);
    // SAFETY: the region is 24 bytes, matching the outer struct layout. The
    // inner struct is inline at offset 8.
    unsafe {
        *region.as_ptr() = 5; // a
        write_u64(region.as_ptr(), 8usize, 100); // inner.x
        write_u64(region.as_ptr(), 16usize, 200); // inner.y
    }

    let bytes = unsafe { serialize_value(region.as_ptr(), outer_ty) }.unwrap();
    let oracle = Outer {
        a: 5,
        inner: Inner { x: 100, y: 200 },
    };
    assert_eq!(bytes, bcs::to_bytes(&oracle).unwrap());
}

#[test]
fn reference_type_is_unsupported() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let ref_ty = guard.immut_ref_of(U64_TY);

    let region = MemoryRegion::new(16);
    let err = unsafe { serialize_value(region.as_ptr(), ref_ty) }.unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::ValueSerialization(ValueSerializationError::UnsupportedType)
    ));
}

#[test]
fn enum_layout_is_unsupported() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let module_id = guard.module_id_of(&AccountAddress::ONE, ident_str!("m"));
    let enum_ty = guard.intern_nominal(
        module_id,
        guard.identifier_of(ident_str!("E")),
        EMPTY_TYPE_LIST,
    );
    // An enum layout carries no per-field offsets.
    guard.set_nominal_layout(enum_ty, 8, 8, None).unwrap();

    let region = MemoryRegion::new(8);
    let err = unsafe { serialize_value(region.as_ptr(), enum_ty) }.unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::ValueSerialization(ValueSerializationError::UnsupportedType)
    ));
}

#[test]
fn non_null_empty_vector() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let vec_ty = guard.vector_of(U64_TY);

    // A live, non-null vector object whose length is 0.
    let data = vec_region(0, &[]);
    let slot = ptr_slot(data.as_ptr());
    let bytes = unsafe { serialize_value(slot.as_ptr(), vec_ty) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&Vec::<u64>::new()).unwrap());
}

#[test]
fn vector_of_struct_is_inline_elements() {
    #[derive(Serialize)]
    struct P {
        a: u8,
        b: u64,
    }

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let module_id = guard.module_id_of(&AccountAddress::ONE, ident_str!("m"));

    // P { a: u8 @0, b: u64 @8 }, size 16.
    let p_ty = guard.intern_nominal(
        module_id,
        guard.identifier_of(ident_str!("P")),
        EMPTY_TYPE_LIST,
    );
    let p_fields = [FieldLayout::new(0, U8_TY), FieldLayout::new(8, U64_TY)];
    guard
        .set_nominal_layout(p_ty, 16, 8, Some(&p_fields))
        .unwrap();
    let vec_ty = guard.vector_of(p_ty);

    // Two inline 16-byte elements; padding between `a` and `b` is dropped.
    let mut elem_bytes = vec![0u8; 32];
    elem_bytes[0] = 1;
    elem_bytes[8..16].copy_from_slice(&1000u64.to_le_bytes());
    elem_bytes[16] = 2;
    elem_bytes[24..32].copy_from_slice(&2000u64.to_le_bytes());
    let data = vec_region(2, &elem_bytes);
    let slot = ptr_slot(data.as_ptr());

    let bytes = unsafe { serialize_value(slot.as_ptr(), vec_ty) }.unwrap();
    let oracle = vec![P { a: 1, b: 1000 }, P { a: 2, b: 2000 }];
    assert_eq!(bytes, bcs::to_bytes(&oracle).unwrap());
}

#[test]
fn vector_of_vector_is_pointer_elements() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let inner_ty = guard.vector_of(U64_TY);
    let outer_ty = guard.vector_of(inner_ty);

    let inner0 = vec_region(2, &u64_bytes(&[1, 2]));
    let inner1 = vec_region(1, &u64_bytes(&[3]));

    // Outer data region: [len = 2 | ptr0 | ptr1].
    let outer = MemoryRegion::new(VEC_DATA_OFFSET + 16);
    // SAFETY: the region holds a length and two 8-byte pointers.
    unsafe {
        write_u64(outer.as_ptr(), 0usize, 2);
        write_ptr(outer.as_ptr(), VEC_DATA_OFFSET, inner0.as_ptr());
        write_ptr(outer.as_ptr(), VEC_DATA_OFFSET + 8, inner1.as_ptr());
    }
    let slot = ptr_slot(outer.as_ptr());

    let bytes = unsafe { serialize_value(slot.as_ptr(), outer_ty) }.unwrap();
    let oracle = vec![vec![1u64, 2], vec![3u64]];
    assert_eq!(bytes, bcs::to_bytes(&oracle).unwrap());
}

#[test]
fn generic_struct_instance_serializes() {
    #[derive(Serialize)]
    struct G {
        v: u64,
    }

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let module_id = guard.module_id_of(&AccountAddress::ONE, ident_str!("m"));

    // A `Nominal` carrying type arguments, i.e. a generic instance `G<u64>`.
    let ty_args = guard.type_list_of(&[U64_TY]);
    let g_ty = guard.intern_nominal(module_id, guard.identifier_of(ident_str!("G")), ty_args);
    guard
        .set_nominal_layout(g_ty, 8, 8, Some(&[FieldLayout::new(0, U64_TY)]))
        .unwrap();

    let region = region_from_bytes(&77u64.to_le_bytes());
    let bytes = unsafe { serialize_value(region.as_ptr(), g_ty) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&G { v: 77 }).unwrap());
}

#[test]
fn zero_field_struct_is_empty() {
    #[derive(Serialize)]
    struct Empty {}

    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let module_id = guard.module_id_of(&AccountAddress::ONE, ident_str!("m"));
    let e_ty = guard.intern_nominal(
        module_id,
        guard.identifier_of(ident_str!("Empty")),
        EMPTY_TYPE_LIST,
    );
    guard.set_nominal_layout(e_ty, 0, 1, Some(&[])).unwrap();

    let region = MemoryRegion::new(1);
    let bytes = unsafe { serialize_value(region.as_ptr(), e_ty) }.unwrap();
    assert_eq!(bytes, bcs::to_bytes(&Empty {}).unwrap());
    assert!(bytes.is_empty());
}

#[test]
fn sequence_too_long_errors() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let vec_ty = guard.vector_of(U64_TY);

    // A header claiming more than BCS's maximum sequence length. We error
    // before touching any element data, so no element region is needed.
    let header = MemoryRegion::new(VEC_DATA_OFFSET);
    // SAFETY: the region is exactly the length field.
    unsafe { write_u64(header.as_ptr(), 0usize, 1u64 << 31) };
    let slot = ptr_slot(header.as_ptr());

    let err = unsafe { serialize_value(slot.as_ptr(), vec_ty) }.unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::ValueSerialization(ValueSerializationError::SequenceTooLong { .. })
    ));
}

#[test]
fn unpopulated_nominal_layout_errors() {
    let ctx = GlobalContext::with_num_execution_workers(1);
    let guard = ctx.try_execution_context(0).unwrap();
    let module_id = guard.module_id_of(&AccountAddress::ONE, ident_str!("m"));
    // Interned but layout never set.
    let ty = guard.intern_nominal(
        module_id,
        guard.identifier_of(ident_str!("Unset")),
        EMPTY_TYPE_LIST,
    );

    let region = MemoryRegion::new(8);
    let err = unsafe { serialize_value(region.as_ptr(), ty) }.unwrap_err();
    assert!(matches!(
        err,
        RuntimeError::ValueSerialization(ValueSerializationError::LayoutUnavailable)
    ));
}
