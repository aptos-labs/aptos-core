// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the static verifier (`verify_function`, `verify_descriptors`).

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    Code, CodeOffset as CO, DescriptorId, ExecutableId, FrameLayoutInfo, FrameOffset as FO,
    Function, MicroOp, SortedSafePointEntries,
};
use mono_move_runtime::{verify_function, ObjectDescriptor, ObjectDescriptorTable};
use move_core_types::account_address::AccountAddress;

static VERIFIER_TEST_MODULE_ID_STORAGE: ExecutableId = unsafe {
    // SAFETY: the backing `&'static str` outlives the program; the
    // resulting `GlobalArenaPtr<str>` is valid for that lifetime.
    ExecutableId::new(
        AccountAddress::ONE,
        GlobalArenaPtr::from_static("verifier_test"),
    )
};
const VERIFIER_TEST_MODULE_ID: GlobalArenaPtr<ExecutableId> =
    GlobalArenaPtr::from_static(&VERIFIER_TEST_MODULE_ID_STORAGE);

fn trivial_descriptors() -> ObjectDescriptorTable {
    let mut t = ObjectDescriptorTable::new();
    t.push(ObjectDescriptor::new_vector(8, vec![]).unwrap());
    t
}

/// A minimal well-formed function: one `Return`, param_and_local_sizes_sum 8.
fn minimal_func() -> Function {
    Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![MicroOp::Return]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 8,
        extended_frame_size: 32,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

// ---------------------------------------------------------------------------
// Positive: well-formed programs pass cleanly
// ---------------------------------------------------------------------------

#[test]
fn valid_minimal() {
    let func = minimal_func();
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

#[test]
fn valid_with_arithmetic_and_jumps() {
    use MicroOp::*;

    #[rustfmt::skip]
    let code = vec![
        StoreImm8 { dst: FO(0), imm: 10 },
        StoreImm8 { dst: FO(8), imm: 1 },
        SubU64Imm { dst: FO(0), src: FO(0), imm: 1 },
        JumpNotZeroU64 { target: CO(2), src: FO(0) },
        Return,
    ];
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(code),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 16,
        extended_frame_size: 40,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

#[test]
fn valid_with_vec_and_pointer_slots() {
    use MicroOp::*;

    #[rustfmt::skip]
    let code = vec![
        VecNew { dst: FO(0) },
        SlotBorrow { dst: FO(16), local: FO(0) },
        StoreImm8 { dst: FO(8), imm: 42 },
        VecPushBack { vec_ref: FO(16), elem: FO(8), elem_size: 8, descriptor_id: DescriptorId(2) },
        Return,
    ];
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(code),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 32,
        extended_frame_size: 56,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

// ---------------------------------------------------------------------------
// Frame bounds violations
// ---------------------------------------------------------------------------

#[test]
fn frame_bounds_store_u64() {
    use MicroOp::*;
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![StoreImm8 { dst: FO(8), imm: 0 }, Return]),
        param_sizes: vec![],
        param_and_local_sizes_sum: 8,
        extended_frame_size: 32, // offset 8 lands in metadata [8, 32)
        param_sizes_sum: 0,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
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
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            Move {
                dst: FO(8),
                src: FO(0),
                size: 16,
            },
            Return,
        ]),
        param_and_local_sizes_sum: 16,
        extended_frame_size: 40, // dst [8, 24) overlaps metadata [16, 40)
        param_sizes: vec![],
        param_sizes_sum: 0,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn frame_bounds_fat_ptr_write() {
    use MicroOp::*;
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            StoreImm8 { dst: FO(0), imm: 0 },
            SlotBorrow {
                dst: FO(8),
                local: FO(0),
            },
            Return,
        ]),
        param_and_local_sizes_sum: 16,
        extended_frame_size: 40, // dst [8, 24) overlaps metadata [16, 40)
        param_sizes: vec![],
        param_sizes_sum: 0,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn frame_bounds_callfunc_metadata() {
    use MicroOp::*;
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            CallIndirect {
                executable_id: VERIFIER_TEST_MODULE_ID,
                func_name: GlobalArenaPtr::from_static("fn_1"),
            },
            Return,
        ]),
        param_sizes: vec![],
        param_and_local_sizes_sum: 8,
        extended_frame_size: 16, // param_and_local_sizes_sum 8 + 24 = 32 > 16
        param_sizes_sum: 0,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
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
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![MicroOp::Return]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 8,
        extended_frame_size: 32,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(100)]), // offset 100 + 8 > extended_frame_size 32
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("exceeds extended_frame_size")));
}

#[test]
fn pointer_slots_overlaps_metadata() {
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![MicroOp::Return]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 8,
        extended_frame_size: 40,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(8)]), // offset 8 overlaps metadata [8, 32) since param_and_local_sizes_sum = 8
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn param_and_local_sizes_sum_misaligned() {
    // SAFETY: Arena is alive for the duration of the test.
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![MicroOp::Return]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 1, // not a multiple of 8
        extended_frame_size: 32,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("8-byte aligned")));
}

#[test]
fn args_size_exceeds_data_size() {
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![MicroOp::Return]),
        param_sizes: vec![],
        param_and_local_sizes_sum: 8,
        extended_frame_size: 32,
        param_sizes_sum: 16, // > param_and_local_sizes_sum 8
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("param_sizes_sum")));
}

// ---------------------------------------------------------------------------
// Jump target out of bounds
// ---------------------------------------------------------------------------

#[test]
fn invalid_jump_target() {
    use MicroOp::*;
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            Jump { target: CO(5) }, // only 2 instructions -> 5 >= 2
            Return,
        ]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 8,
        extended_frame_size: 32,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("jump target")));
}

#[test]
fn invalid_conditional_jump_target() {
    use MicroOp::*;
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            StoreImm8 { dst: FO(0), imm: 0 },
            JumpNotZeroU64 {
                target: CO(99),
                src: FO(0),
            },
            Return,
        ]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 8,
        extended_frame_size: 32,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
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
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            CallIndirect {
                executable_id: VERIFIER_TEST_MODULE_ID,
                func_name: GlobalArenaPtr::from_static("fn_42"),
            },
            Return,
        ]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 0,
        extended_frame_size: 32,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("func_id")));
}

// ---------------------------------------------------------------------------
// Invalid descriptor ID
// ---------------------------------------------------------------------------

#[test]
fn invalid_descriptor_id() {
    use MicroOp::*;

    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
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
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 32,
        extended_frame_size: 56,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("descriptor_id")));
}

// ---------------------------------------------------------------------------
// Nonzero size checks
// ---------------------------------------------------------------------------

#[test]
fn zero_size_mov() {
    use MicroOp::*;
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            Move {
                dst: FO(0),
                src: FO(0),
                size: 0,
            },
            Return,
        ]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 8,
        extended_frame_size: 32,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("size")));
}

#[test]
fn zero_elem_size_vec_push() {
    use MicroOp::*;

    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
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
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 32,
        extended_frame_size: 56,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("size")));
}

// ---------------------------------------------------------------------------
// Function-level sanity
// ---------------------------------------------------------------------------

#[test]
fn empty_code() {
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 8,
        extended_frame_size: 32,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("non-empty")));
}

#[test]
fn zero_frame_size() {
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![MicroOp::Return]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 0,
        extended_frame_size: 0,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("frame_size")));
}

// ---------------------------------------------------------------------------
// Static arithmetic constraints (imm-form ops)
// ---------------------------------------------------------------------------
//
// Some imm-form ops would always abort at runtime for a particular imm
// value (`Div`/`Mod` with `0`, shifts with `>= 64`). The verifier rejects
// these statically.

fn func_with_single_op(op: MicroOp) -> Function {
    Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![op, MicroOp::Return]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 24,
        extended_frame_size: 48,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

#[test]
fn div_u64_imm_zero() {
    let func = func_with_single_op(MicroOp::DivU64Imm {
        dst: FO(0),
        src: FO(8),
        imm: 0,
    });
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("division by zero")),
        "expected division-by-zero error, got: {:?}",
        errors
    );
}

#[test]
fn mod_u64_imm_zero() {
    let func = func_with_single_op(MicroOp::ModU64Imm {
        dst: FO(0),
        src: FO(8),
        imm: 0,
    });
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("division by zero")),
        "expected division-by-zero error, got: {:?}",
        errors
    );
}

#[test]
fn div_u64_imm_nonzero_ok() {
    let func = func_with_single_op(MicroOp::DivU64Imm {
        dst: FO(0),
        src: FO(8),
        imm: 1,
    });
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

#[test]
fn shl_u64_imm_oversize() {
    let func = func_with_single_op(MicroOp::ShlU64Imm {
        dst: FO(0),
        src: FO(8),
        imm: 64,
    });
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(
        errors.iter().any(|e| e.message.contains("shift amount")),
        "expected oversize-shift error, got: {:?}",
        errors
    );
}

#[test]
fn shr_u64_imm_oversize() {
    let func = func_with_single_op(MicroOp::ShrU64Imm {
        dst: FO(0),
        src: FO(8),
        imm: 100,
    });
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(
        errors.iter().any(|e| e.message.contains("shift amount")),
        "expected oversize-shift error, got: {:?}",
        errors
    );
}

#[test]
fn shl_u64_imm_in_range_ok() {
    let func = func_with_single_op(MicroOp::ShlU64Imm {
        dst: FO(0),
        src: FO(8),
        imm: 63,
    });
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

// ---------------------------------------------------------------------------
// Multiple errors collected
// ---------------------------------------------------------------------------

#[test]
fn multiple_errors_collected() {
    use MicroOp::*;
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            StoreImm8 {
                dst: FO(100),
                imm: 0,
            }, // out of bounds
            Jump { target: CO(99) }, // invalid target
            Return,
        ]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 8,
        extended_frame_size: 32,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
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
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
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
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 32,
        extended_frame_size: 56,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("VecPushBack") && e.message.contains("not a Vector")));
}

#[test]
fn heap_new_rejects_vector_descriptor() {
    use MicroOp::*;
    // trivial_descriptors() has Vector at index 2.
    let func = Function {
        name: GlobalArenaPtr::from_static("test"),
        code: Code::from_vec(vec![
            HeapNew {
                dst: FO(0),
                descriptor_id: DescriptorId(2),
            },
            Return,
        ]),
        param_sizes: vec![],
        param_sizes_sum: 0,
        param_and_local_sizes_sum: 8,
        extended_frame_size: 32,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        safe_point_layouts: SortedSafePointEntries::empty(),
    };
    let errors = verify_function(&func, &trivial_descriptors());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("not a Struct or Enum")));
}
