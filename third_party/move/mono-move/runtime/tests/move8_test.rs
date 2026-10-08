// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! The 8-byte move operations (`Move8`, `HeapMoveTo8`, `HeapMoveFrom8`) must
//! preserve pointer provenance and uninitialized padding.

mod common;

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    native::NativeExtensions, types::EMPTY_TYPE_LIST, Code, FrameLayoutInfo, FrameOffset as FO,
    Function, FunctionDefinitionIndex, MicroOp, SortedSafePointEntries, FRAME_METADATA_SIZE,
    POINTER_VEC_DESCRIPTOR_ID, TRIVIAL_DESCRIPTOR_ID, VEC_DATA_OFFSET,
};

fn function(code: Vec<MicroOp>, frame_size: usize, pointer_slots: Vec<FO>) -> Function {
    Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: crate::program_module_id!("test"),
        func_ref: crate::program_func_ref!("test", "test"),
        def_idx: FunctionDefinitionIndex(0),
        code: Code::from_vec(code),
        entry_gas: 0,
        param_slots: vec![],
        param_tys: vec![],
        return_slots: vec![],
        return_tys: EMPTY_TYPE_LIST,
        param_region_size: 0,
        param_and_local_sizes_sum: frame_size,
        extended_frame_size: frame_size + FRAME_METADATA_SIZE,
        zero_frame: !pointer_slots.is_empty(),
        frame_layout: FrameLayoutInfo::new(pointer_slots),
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

/// Builds micro-ops that leave the vector `[7]` at byte offset 8, using bytes
/// 16..40 for scratch.
fn push_one_elem_vec() -> Vec<MicroOp> {
    use MicroOp::*;
    #[rustfmt::skip]
    let code = vec![
        VecNew { dst: FO(8) },
        SlotBorrow { dst: FO(24), local: FO(8) },
        StoreImm8 { dst: FO(16), imm: 7u64.to_le_bytes() },
        VecPushBack { vec_ref: FO(24), elem: FO(16), elem_size: 8, descriptor_id: TRIVIAL_DESCRIPTOR_ID },
    ];
    code
}

/// The copied vector pointer remains usable after the source slot is cleared
/// and GC relocates the vector.
#[test]
fn move8_keeps_heap_pointer_usable() {
    use MicroOp::*;

    let mut code = push_one_elem_vec();
    #[rustfmt::skip]
    code.extend([
        Move8 { dst: FO(40), src: FO(8) },
        VecNew { dst: FO(8) },
        ForceGC,
        SlotBorrow { dst: FO(48), local: FO(40) },
        VecLen { dst: FO(0), vec_ref: FO(48) },
        Return,
    ]);
    let entry = function(code, 64, vec![FO(8), FO(40)]);
    let len =
        common::with_test_interpreter(&entry, u64::MAX, NativeExtensions::new(), |ctx, entry| {
            ctx.build_call(entry).unwrap().run().unwrap();
            ctx.root_result_u64_for_test()
        });
    assert_eq!(len, 1);
}

/// A vector pointer stored by `HeapMoveTo8` remains reachable through the outer
/// vector during GC and usable after `HeapMoveFrom8`.
#[test]
fn heap_move8_keeps_heap_pointer_usable() {
    use MicroOp::*;

    let elem0 = VEC_DATA_OFFSET as u32;
    let mut code = push_one_elem_vec();
    #[rustfmt::skip]
    code.extend([
        // Outer vector holding one null placeholder for `HeapMoveTo8` to overwrite.
        VecNew { dst: FO(40) },
        VecNew { dst: FO(48) },
        SlotBorrow { dst: FO(56), local: FO(40) },
        VecPushBack { vec_ref: FO(56), elem: FO(48), elem_size: 8, descriptor_id: POINTER_VEC_DESCRIPTOR_ID },
        HeapMoveTo8 { heap_ptr: FO(40), offset: elem0, src: FO(8) },
        VecNew { dst: FO(8) },
        ForceGC,
        HeapMoveFrom8 { dst: FO(48), heap_ptr: FO(40), offset: elem0 },
        SlotBorrow { dst: FO(56), local: FO(48) },
        VecLen { dst: FO(0), vec_ref: FO(56) },
        Return,
    ]);
    let entry = function(code, 72, vec![FO(8), FO(40), FO(48)]);
    let len =
        common::with_test_interpreter(&entry, u64::MAX, NativeExtensions::new(), |ctx, entry| {
            ctx.build_call(entry).unwrap().run().unwrap();
            ctx.root_result_u64_for_test()
        });
    assert_eq!(len, 1);
}

/// `Move8` copies an 8-byte `{ a: u32, b: u8 }` value with three padding bytes.
/// The frame has no pointer slots, so it is not zeroed; under Miri, the padding
/// remains uninitialized.
#[test]
fn move8_copies_uninitialized_padding() {
    use MicroOp::*;

    #[rustfmt::skip]
    let code = vec![
        StoreImm4 { dst: FO(8), imm: 1u32.to_le_bytes() },
        StoreImm1 { dst: FO(12), imm: 2 },
        Move8 { dst: FO(16), src: FO(8) },
        Move { dst: FO(0), src: FO(16), size: 5 },
        Return,
    ];
    let entry = function(code, 24, vec![]);
    let bytes =
        common::with_test_interpreter(&entry, u64::MAX, NativeExtensions::new(), |ctx, entry| {
            ctx.build_call(entry).unwrap().run().unwrap();
            ctx.root_result_bytes_for_test(0, 5).to_vec()
        });
    assert_eq!(bytes, [1, 0, 0, 0, 2]);
}
