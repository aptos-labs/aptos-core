// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the static verifier (`verify_function`, `verify_descriptors`).

use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{
    CodeOffset as CO, DescriptorId, FrameLayoutInfo, FrameOffset as FO, Function, MicroOp,
    SortedSafePointEntries,
};
use mono_move_runtime::{verify_function, ObjectDescriptor, ObjectDescriptorTable};

fn trivial_descriptors() -> ObjectDescriptorTable {
    let mut t = ObjectDescriptorTable::new();
    t.push(ObjectDescriptor::new_vector(8, vec![]).unwrap());
    t
}

/// A minimal well-formed function: one `Return`, param_and_local_sizes_sum 8.
fn minimal_func(arena: &ExecutableArena) -> &Function {
    // SAFETY: Arena is alive for the duration of the test.
    unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([MicroOp::Return]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 8,
                extended_frame_size: 32,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    }
}

// ---------------------------------------------------------------------------
// Positive: well-formed programs pass cleanly
// ---------------------------------------------------------------------------

#[test]
fn valid_minimal() {
    let arena = ExecutableArena::new();
    let errors = verify_function(minimal_func(&arena), &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

#[test]
fn valid_with_arithmetic_and_jumps() {
    use MicroOp::*;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        StoreImm8 { dst: FO(0), imm: 10 },
        StoreImm8 { dst: FO(8), imm: 1 },
        SubU64Imm { dst: FO(0), src: FO(0), imm: 1 },
        JumpNotZeroU64 { target: CO(2), src: FO(0) },
        Return,
    ]);
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code,
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 16,
                extended_frame_size: 40,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

#[test]
fn valid_with_vec_and_pointer_slots() {
    use MicroOp::*;

    let arena = ExecutableArena::new();

    #[rustfmt::skip]
    let code = arena.alloc_slice_fill_iter([
        VecNew { dst: FO(0) },
        SlotBorrow { dst: FO(16), local: FO(0) },
        StoreImm8 { dst: FO(8), imm: 42 },
        VecPushBack { vec_ref: FO(16), elem: FO(8), elem_size: 8, descriptor_id: DescriptorId(2) },
        Return,
    ]);
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code,
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 32,
                extended_frame_size: 56,
                zero_frame: true,
                frame_layout: FrameLayoutInfo::new(&arena, [FO(0)]),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

// ---------------------------------------------------------------------------
// Frame bounds violations
// ---------------------------------------------------------------------------

#[test]
fn frame_bounds_store_u64() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([StoreImm8 { dst: FO(8), imm: 0 }, Return]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_and_local_sizes_sum: 8,
                extended_frame_size: 32, // offset 8 lands in metadata [8, 32)
                param_sizes_sum: 0,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].message.contains("overlaps metadata"),
        "{}",
        errors[0]
    );
}

#[test]
fn frame_bounds_mov() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([
                    Move {
                        dst: FO(8),
                        src: FO(0),
                        size: 16,
                    },
                    Return,
                ]),
                param_and_local_sizes_sum: 16,
                extended_frame_size: 40, // dst [8, 24) overlaps metadata [16, 40)
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn frame_bounds_fat_ptr_write() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([
                    StoreImm8 { dst: FO(0), imm: 0 },
                    SlotBorrow {
                        dst: FO(8),
                        local: FO(0),
                    },
                    Return,
                ]),
                param_and_local_sizes_sum: 16,
                extended_frame_size: 40, // dst [8, 24) overlaps metadata [16, 40)
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn frame_bounds_callfunc_metadata() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([CallFunc { func_id: 1 }, Return]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_and_local_sizes_sum: 8,
                extended_frame_size: 16, // param_and_local_sizes_sum 8 + 24 = 32 > 16
                param_sizes_sum: 0,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("extended_frame_size") && e.message.contains("frame_size()")));
}

// ---------------------------------------------------------------------------
// Pointer slots validation
// ---------------------------------------------------------------------------

#[test]
fn pointer_slots_offset_out_of_bounds() {
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([MicroOp::Return]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 8,
                extended_frame_size: 32,
                zero_frame: true,
                frame_layout: FrameLayoutInfo::new(&arena, [FO(100)]), // offset 100 + 8 > extended_frame_size 32
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("exceeds extended_frame_size")));
}

#[test]
fn pointer_slots_overlaps_metadata() {
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([MicroOp::Return]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 8,
                extended_frame_size: 40,
                zero_frame: true,
                frame_layout: FrameLayoutInfo::new(&arena, [FO(8)]), // offset 8 overlaps metadata [8, 32) since param_and_local_sizes_sum = 8
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn args_size_exceeds_data_size() {
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([MicroOp::Return]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_and_local_sizes_sum: 8,
                extended_frame_size: 32,
                param_sizes_sum: 16, // > param_and_local_sizes_sum 8
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("param_sizes_sum")));
}

// ---------------------------------------------------------------------------
// Jump target out of bounds
// ---------------------------------------------------------------------------

#[test]
fn invalid_jump_target() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([
                    Jump { target: CO(5) }, // only 2 instructions -> 5 >= 2
                    Return,
                ]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 8,
                extended_frame_size: 32,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("jump target")));
}

#[test]
fn invalid_conditional_jump_target() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([
                    StoreImm8 { dst: FO(0), imm: 0 },
                    JumpNotZeroU64 {
                        target: CO(99),
                        src: FO(0),
                    },
                    Return,
                ]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 8,
                extended_frame_size: 32,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("jump target")));
}

// ---------------------------------------------------------------------------
// Invalid cross-function references
// ---------------------------------------------------------------------------

#[test]
#[ignore]
fn invalid_callfunc_func_id() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([CallFunc { func_id: 42 }, Return]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 0,
                extended_frame_size: 32,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("func_id")));
}

// ---------------------------------------------------------------------------
// Invalid descriptor ID
// ---------------------------------------------------------------------------

#[test]
fn invalid_descriptor_id() {
    use MicroOp::*;

    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([
                    VecNew { dst: FO(0) },
                    SlotBorrow {
                        dst: FO(8),
                        local: FO(0),
                    },
                    StoreImm8 {
                        dst: FO(24),
                        imm: 42,
                    },
                    VecPushBack {
                        vec_ref: FO(8),
                        elem: FO(24),
                        elem_size: 8,
                        descriptor_id: DescriptorId(99),
                    },
                    Return,
                ]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 32,
                extended_frame_size: 56,
                zero_frame: true,
                frame_layout: FrameLayoutInfo::new(&arena, [FO(0)]),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("descriptor_id")));
}

// ---------------------------------------------------------------------------
// Nonzero size checks
// ---------------------------------------------------------------------------

#[test]
fn zero_size_mov() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([
                    Move {
                        dst: FO(0),
                        src: FO(0),
                        size: 0,
                    },
                    Return,
                ]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 8,
                extended_frame_size: 32,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("size")));
}

#[test]
fn zero_elem_size_vec_push() {
    use MicroOp::*;

    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([
                    VecNew { dst: FO(0) },
                    SlotBorrow {
                        dst: FO(8),
                        local: FO(0),
                    },
                    StoreImm8 {
                        dst: FO(24),
                        imm: 42,
                    },
                    VecPushBack {
                        vec_ref: FO(8),
                        elem: FO(24),
                        elem_size: 0,
                        descriptor_id: DescriptorId(0),
                    },
                    Return,
                ]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 32,
                extended_frame_size: 56,
                zero_frame: true,
                frame_layout: FrameLayoutInfo::new(&arena, [FO(0)]),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("size")));
}

// ---------------------------------------------------------------------------
// Function-level sanity
// ---------------------------------------------------------------------------

#[test]
fn empty_code() {
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: ExecutableArenaPtr::empty_slice(),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 8,
                extended_frame_size: 32,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("non-empty")));
}

#[test]
fn zero_frame_size() {
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([MicroOp::Return]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 0,
                extended_frame_size: 0,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("frame_size")));
}

// ---------------------------------------------------------------------------
// Multiple errors collected
// ---------------------------------------------------------------------------

#[test]
fn multiple_errors_collected() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // SAFETY: Arena is alive for the duration of the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([
                    StoreImm8 {
                        dst: FO(100),
                        imm: 0,
                    }, // out of bounds
                    Jump { target: CO(99) }, // invalid target
                    Return,
                ]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 8,
                extended_frame_size: 32,
                zero_frame: false,
                frame_layout: FrameLayoutInfo::empty(),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(
        errors.len() >= 2,
        "expected at least 2 errors, got {}",
        errors.len()
    );
}

// ---------------------------------------------------------------------------
// Op/variant tightening
//
// Descriptor table self-soundness (reserved indices, nonzero sizes,
// in-bounds pointer offsets, etc.) is now enforced structurally by
// `ObjectDescriptorTable`; see its unit tests in `runtime/src/types.rs`.
// ---------------------------------------------------------------------------

#[test]
fn vec_pushback_must_target_vector_descriptor() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // SAFETY: arena outlives the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([
                    VecNew { dst: FO(0) },
                    SlotBorrow {
                        dst: FO(16),
                        local: FO(0),
                    },
                    StoreImm8 { dst: FO(8), imm: 1 },
                    VecPushBack {
                        vec_ref: FO(16),
                        elem: FO(8),
                        elem_size: 8,
                        descriptor_id: DescriptorId(0), // Trivial — wrong variant
                    },
                    Return,
                ]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 32,
                extended_frame_size: 56,
                zero_frame: true,
                frame_layout: FrameLayoutInfo::new(&arena, [FO(0)]),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("VecPushBack") && e.message.contains("not a Vector")));
}

#[test]
fn heap_new_rejects_vector_descriptor() {
    use MicroOp::*;
    let arena = ExecutableArena::new();
    // trivial_descriptors() has Vector at index 2.
    // SAFETY: arena outlives the test.
    let func = unsafe {
        arena
            .alloc(Function {
                name: GlobalArenaPtr::from_static("test"),
                code: arena.alloc_slice_fill_iter([
                    HeapNew {
                        dst: FO(0),
                        descriptor_id: DescriptorId(2),
                    },
                    Return,
                ]),
                param_sizes: ExecutableArenaPtr::empty_slice(),
                param_sizes_sum: 0,
                param_and_local_sizes_sum: 8,
                extended_frame_size: 32,
                zero_frame: true,
                frame_layout: FrameLayoutInfo::new(&arena, [FO(0)]),
                safe_point_layouts: SortedSafePointEntries::empty(),
            })
            .as_ref_unchecked()
    };
    let errors = verify_function(func, &trivial_descriptors());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("not a Struct or Enum")));
}
