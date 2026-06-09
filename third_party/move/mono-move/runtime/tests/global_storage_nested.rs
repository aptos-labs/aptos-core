// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Integration tests for `BorrowGlobalMut` / `MoveFrom` on resources
//! that contain sub-allocations. Exercise every arm of
//! `walk_children` in `global_storage::deep_copy_value`:
//!   - Struct  → child pointer field (parent is a struct).
//!   - Vector  → element pointers (parent struct points at a vector
//!     whose elements are pointers to other heap objects).
//!   - Enum    → variant pointer fields (parent is an enum).
//!
//! Each test installs the resource graph as a chain of anchored
//! allocations in `InMemoryResources`, runs `BorrowGlobalMut` to
//! force a deep copy, then asserts:
//!   - the local-heap root differs from the storage root,
//!   - the local copy traces to a *fresh* sub-allocation (also
//!     different from the storage sub-allocation),
//!   - the contents survive the copy.

mod common;

use common::InMemoryResources;
use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    types::{InternedType, Type},
    Code, DescriptorId, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp, ObjectDescriptor,
    ObjectDescriptorTable, SortedSafePointEntries, ENUM_DATA_OFFSET, ENUM_TAG_OFFSET,
};
use mono_move_gas::SimpleGasMeter;
use mono_move_runtime::{
    read_ptr, read_u64, InterpreterContext, LocalRuntimeContext, VEC_DATA_OFFSET, VEC_LENGTH_OFFSET,
};
use move_core_types::account_address::AccountAddress;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

static RESOURCE_TY_NODE: Type = Type::U64;

fn resource_ty() -> InternedType {
    GlobalArenaPtr::from_static(&RESOURCE_TY_NODE)
}

fn addr(byte: u8) -> AccountAddress {
    let mut bytes = [0u8; AccountAddress::LENGTH];
    bytes[AccountAddress::LENGTH - 1] = byte;
    AccountAddress::new(bytes)
}

fn local_ctx_with<'r>(
    resources: &'r InMemoryResources,
    descriptors: ObjectDescriptorTable,
) -> LocalRuntimeContext<'r, SimpleGasMeter> {
    LocalRuntimeContext::new(SimpleGasMeter::new(u64::MAX), resources, descriptors)
}

/// Frame layout: 32B addr at 8, 16B tmp at 40 (a mutable borrow writes a
/// 16-byte fat-pointer reference). Locals total 56 bytes; with
/// FRAME_METADATA_SIZE (24), extended_frame_size must be ≥ 80.
const ADDR: FO = FO(8);
const TMP: FO = FO(40);

fn make_borrow_mut_program(desc_id: DescriptorId) -> Function {
    let _ = desc_id; // descriptor_id is no longer carried on the MicroOp.
    Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(vec![
            MicroOp::BorrowGlobalMut {
                addr: ADDR,
                ty: resource_ty(),
                dst: TMP,
            },
            MicroOp::Return,
        ]),
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 56,
        extended_frame_size: 80,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![TMP]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

/// Layout for an 8-byte u64 vector storage payload: `[length(8) |
/// data(8 * n)]`. Returns the raw bytes to feed `install_anchor`.
fn build_u64_vec_payload(values: &[u64]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(VEC_DATA_OFFSET + values.len() * 8);
    buf.extend_from_slice(&(values.len() as u64).to_le_bytes());
    debug_assert_eq!(buf.len(), VEC_DATA_OFFSET);
    for v in values {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    buf
}

// ===========================================================================
// Struct → Struct (nested struct)
// ===========================================================================

/// Parent: `Struct { inner: ptr }` (8B, pointer at offset 0).
/// Inner:  `Struct { value: u64 }` (8B, no pointers).
#[test]
fn borrow_global_mut_deep_copies_nested_struct() {
    let mut descriptors = ObjectDescriptorTable::new();
    let inner_desc = descriptors.push(ObjectDescriptor::new_struct(8, vec![]).unwrap());
    let parent_desc = descriptors.push(ObjectDescriptor::new_struct(8, vec![0]).unwrap());

    let resources = InMemoryResources::new();
    let inner_storage_ptr = resources.install_anchor(inner_desc, &0xCAFE_u64.to_le_bytes());
    let parent_payload = (inner_storage_ptr as u64).to_le_bytes();
    let parent_storage_ptr = resources.install_anchor(parent_desc, &parent_payload);
    // Wire the parent into the working-map key.
    resources.entries_install(addr(1), resource_ty(), parent_storage_ptr);

    let mut exec_ctx = local_ctx_with(&resources, descriptors);
    let func = make_borrow_mut_program(parent_desc);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(1).into_bytes());
    ctx.run().unwrap();

    let local_parent_ptr = ctx.root_heap_ptr(40);
    assert_ne!(local_parent_ptr as usize, parent_storage_ptr as usize);

    // Local parent's pointer field must point at a *fresh* copy of
    // the inner — different from the storage inner.
    let local_inner_ptr = unsafe { read_ptr(local_parent_ptr, 0_usize) };
    assert_ne!(
        local_inner_ptr as usize, inner_storage_ptr as usize,
        "deep copy must allocate a fresh inner — got the storage pointer"
    );
    let local_inner_val = unsafe { read_u64(local_inner_ptr, 0_usize) };
    assert_eq!(local_inner_val, 0xCAFE);
}

// ===========================================================================
// Struct → Vector (parent struct contains a vector field)
// ===========================================================================

/// Parent: `Struct { vec_ptr }` (8B, pointer at offset 0).
/// Vector elements: u64 (no internal pointers).
#[test]
fn borrow_global_mut_deep_copies_struct_with_vector() {
    let mut descriptors = ObjectDescriptorTable::new();
    let vec_desc = descriptors.push(ObjectDescriptor::new_vector(8, vec![]).unwrap());
    let parent_desc = descriptors.push(ObjectDescriptor::new_struct(8, vec![0]).unwrap());

    let resources = InMemoryResources::new();
    let vec_payload = build_u64_vec_payload(&[0xAAAA, 0xBBBB, 0xCCCC]);
    let vec_storage_ptr = resources.install_anchor(vec_desc, &vec_payload);
    let parent_payload = (vec_storage_ptr as u64).to_le_bytes();
    let parent_storage_ptr = resources.install_anchor(parent_desc, &parent_payload);
    resources.entries_install(addr(2), resource_ty(), parent_storage_ptr);

    let mut exec_ctx = local_ctx_with(&resources, descriptors);
    let func = make_borrow_mut_program(parent_desc);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(2).into_bytes());
    ctx.run().unwrap();

    let local_parent_ptr = ctx.root_heap_ptr(40);
    assert_ne!(local_parent_ptr as usize, parent_storage_ptr as usize);

    let local_vec_ptr = unsafe { read_ptr(local_parent_ptr, 0_usize) };
    assert_ne!(
        local_vec_ptr as usize, vec_storage_ptr as usize,
        "deep copy must allocate a fresh vector — got the storage pointer"
    );
    let local_len = unsafe { read_u64(local_vec_ptr, VEC_LENGTH_OFFSET) };
    assert_eq!(local_len, 3);
    for (i, &expected) in [0xAAAA_u64, 0xBBBB, 0xCCCC].iter().enumerate() {
        let elem = unsafe { read_u64(local_vec_ptr, VEC_DATA_OFFSET + i * 8) };
        assert_eq!(elem, expected);
    }
}

// ===========================================================================
// Enum with a pointer-bearing variant
// ===========================================================================

/// Two-variant enum:
///   - variant 0: empty (no pointers).
///   - variant 1: contains one heap pointer at variant offset 0
///     (data-region offset `ENUM_DATA_OFFSET + 0`).
///
/// Total enum size: 8 (tag) + 8 (one pointer slot, the max variant) = 16.
/// We install variant 1 pointing at an inner Struct holding `0xBEEF`.
#[test]
fn borrow_global_mut_deep_copies_enum_variant() {
    let mut descriptors = ObjectDescriptorTable::new();
    let inner_desc = descriptors.push(ObjectDescriptor::new_struct(8, vec![]).unwrap());
    let enum_desc =
        descriptors.push(ObjectDescriptor::new_enum(16, vec![vec![], vec![0]]).unwrap());

    let resources = InMemoryResources::new();
    let inner_storage_ptr = resources.install_anchor(inner_desc, &0xBEEF_u64.to_le_bytes());

    // Payload: [tag = 1 (u64)] [inner_ptr (8B)]
    let mut payload = Vec::with_capacity(16);
    payload.extend_from_slice(&1u64.to_le_bytes());
    payload.extend_from_slice(&(inner_storage_ptr as u64).to_le_bytes());
    let enum_storage_ptr = resources.install_anchor(enum_desc, &payload);
    resources.entries_install(addr(3), resource_ty(), enum_storage_ptr);

    let mut exec_ctx = local_ctx_with(&resources, descriptors);
    let func = make_borrow_mut_program(enum_desc);
    let mut ctx = InterpreterContext::new(&mut exec_ctx, &func);
    ctx.set_root_arg(8, &addr(3).into_bytes());
    ctx.run().unwrap();

    let local_enum_ptr = ctx.root_heap_ptr(40);
    assert_ne!(local_enum_ptr as usize, enum_storage_ptr as usize);

    let local_tag = unsafe { read_u64(local_enum_ptr, ENUM_TAG_OFFSET) };
    assert_eq!(local_tag, 1);

    let local_inner_ptr = unsafe { read_ptr(local_enum_ptr, ENUM_DATA_OFFSET) };
    assert_ne!(
        local_inner_ptr as usize, inner_storage_ptr as usize,
        "enum variant pointer must be deep-copied"
    );
    let local_inner_val = unsafe { read_u64(local_inner_ptr, 0_usize) };
    assert_eq!(local_inner_val, 0xBEEF);
}

// ===========================================================================
// GC mid-deep-copy
// ===========================================================================

/// Exercise the "GC fires while `deep_copy_value` is recursing"
/// path. Pre-fills a tight heap with unrooted garbage via repeated
/// `HeapNew` into a slot intentionally *outside* `frame_layout`;
/// when the deep-copy then tries to allocate its inner child, GC
/// runs, the garbage is reclaimed, and the alloc succeeds. We
/// assert the resulting structure (root pointer, inner value) is
/// preserved across the relocations.
#[test]
fn deep_copy_survives_gc_mid_walk() {
    let mut descriptors = ObjectDescriptorTable::new();
    let inner_desc = descriptors.push(ObjectDescriptor::new_struct(8, vec![]).unwrap());
    let parent_desc = descriptors.push(ObjectDescriptor::new_struct(8, vec![0]).unwrap());

    let resources = InMemoryResources::new();
    let inner_storage_ptr = resources.install_anchor(inner_desc, &0x9999_u64.to_le_bytes());
    let parent_storage_ptr =
        resources.install_anchor(parent_desc, &(inner_storage_ptr as u64).to_le_bytes());
    resources.entries_install(addr(7), resource_ty(), parent_storage_ptr);

    // Frame: ADDR(32B)@8, TMP(8B)@40 (rooted), GARBAGE(8B)@48 (unrooted).
    // 56 bytes of locals + 24B metadata = 80B extended frame.
    const GARBAGE: FO = FO(48);

    let func = Function {
        name: GlobalArenaPtr::from_static("test_gc_mid_copy"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(vec![
            // Three garbage allocations into the same unrooted slot.
            // Each HeapNew is 16B (header + 8B payload). With heap=32,
            // the third HeapNew already triggers one GC pass.
            MicroOp::HeapNew {
                dst: GARBAGE,
                descriptor_id: inner_desc,
            },
            MicroOp::HeapNew {
                dst: GARBAGE,
                descriptor_id: inner_desc,
            },
            MicroOp::HeapNew {
                dst: GARBAGE,
                descriptor_id: inner_desc,
            },
            // Now deep-copy a 2-level resource. The inner alloc
            // inside `deep_copy_value` won't fit alongside the new
            // parent + garbage; it triggers a second GC that
            // reclaims the garbage and lets the recursion finish.
            MicroOp::BorrowGlobalMut {
                addr: ADDR,
                ty: resource_ty(),
                dst: TMP,
            },
            MicroOp::Return,
        ]),
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 56,
        extended_frame_size: 80,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![TMP]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };

    let mut exec_ctx = local_ctx_with(&resources, descriptors);
    let mut ctx = InterpreterContext::with_heap_size(&mut exec_ctx, &func, 32);
    ctx.set_root_arg(8, &addr(7).into_bytes());
    ctx.run().unwrap();

    assert!(
        ctx.gc_count() >= 1,
        "expected at least one GC during the run; got {}",
        ctx.gc_count()
    );

    let local_parent_ptr = ctx.root_heap_ptr(40);
    assert_ne!(local_parent_ptr as usize, parent_storage_ptr as usize);
    let local_inner_ptr = unsafe { read_ptr(local_parent_ptr, 0_usize) };
    assert_ne!(
        local_inner_ptr as usize, inner_storage_ptr as usize,
        "inner pointer must point at a freshly-allocated local copy after GC"
    );
    let local_inner_val = unsafe { read_u64(local_inner_ptr, 0_usize) };
    assert_eq!(local_inner_val, 0x9999, "inner value preserved across GC");
}

/// MoveFrom counterpart of [`deep_copy_survives_gc_mid_walk`].
///
/// `move_from` flips `entry.write` to `Deleted` and journals the old
/// write before `deep_copy` is invoked, so the rws entry no longer
/// holds `src`. The pin slot in `pinned_roots` is what keeps `src`
/// alive across the wrapper's GC retry. This test puts the heap
/// under enough pressure that the inner allocation OOMs the first
/// `try_deep_copy`, forcing the wrapper to GC and retry through the
/// pin.
#[test]
fn move_from_deep_copies_external_resource_survives_gc_mid_walk() {
    let mut descriptors = ObjectDescriptorTable::new();
    let inner_desc = descriptors.push(ObjectDescriptor::new_struct(8, vec![]).unwrap());
    let parent_desc = descriptors.push(ObjectDescriptor::new_struct(8, vec![0]).unwrap());

    let resources = InMemoryResources::new();
    let inner_storage_ptr = resources.install_anchor(inner_desc, &0xAAAA_u64.to_le_bytes());
    let parent_storage_ptr =
        resources.install_anchor(parent_desc, &(inner_storage_ptr as u64).to_le_bytes());
    resources.entries_install(addr(8), resource_ty(), parent_storage_ptr);

    // Frame: ADDR(32B)@8, TMP(8B)@40 (rooted), GARBAGE(8B)@48 (unrooted).
    // 56 bytes of locals + 24B metadata = 80B extended frame.
    const GARBAGE: FO = FO(48);

    let func = Function {
        name: GlobalArenaPtr::from_static("test_move_from_gc_mid_copy"),
        module_id: crate::program_module_id!("test"),
        code: Code::from_vec(vec![
            // Three garbage allocations into the same unrooted slot —
            // same pressure pattern as the BorrowGlobalMut variant.
            MicroOp::HeapNew {
                dst: GARBAGE,
                descriptor_id: inner_desc,
            },
            MicroOp::HeapNew {
                dst: GARBAGE,
                descriptor_id: inner_desc,
            },
            MicroOp::HeapNew {
                dst: GARBAGE,
                descriptor_id: inner_desc,
            },
            // MoveFrom flips the entry to Deleted, then takes the
            // deep-copy slow path. The inner alloc inside
            // `try_deep_copy` won't fit alongside the freshly
            // allocated parent + garbage; the wrapper's GC retry
            // reclaims everything and the second attempt succeeds.
            // The journal's `LocalHeap` write is irrelevant here —
            // the rws entry no longer carries `src`, so the pin in
            // `pinned_roots` is what survives the GC.
            MicroOp::MoveFrom {
                addr: ADDR,
                ty: resource_ty(),
                dst: TMP,
            },
            MicroOp::Return,
        ]),
        param_slots: vec![],
        param_region_size: 0,
        param_and_local_sizes_sum: 56,
        extended_frame_size: 80,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![TMP]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };

    let mut exec_ctx = local_ctx_with(&resources, descriptors);
    let mut ctx = InterpreterContext::with_heap_size(&mut exec_ctx, &func, 32);
    ctx.set_root_arg(8, &addr(8).into_bytes());
    ctx.run().unwrap();

    assert!(
        ctx.gc_count() >= 1,
        "expected at least one GC during the run; got {}",
        ctx.gc_count()
    );

    let local_parent_ptr = ctx.root_heap_ptr(40);
    assert_ne!(local_parent_ptr as usize, parent_storage_ptr as usize);
    let local_inner_ptr = unsafe { read_ptr(local_parent_ptr, 0_usize) };
    assert_ne!(
        local_inner_ptr as usize, inner_storage_ptr as usize,
        "inner pointer must point at a freshly-allocated local copy after GC"
    );
    let local_inner_val = unsafe { read_u64(local_inner_ptr, 0_usize) };
    assert_eq!(local_inner_val, 0xAAAA, "inner value preserved across GC");
}
