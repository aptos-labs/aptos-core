// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the static verifier (`verify_program`).

use mono_move_runtime::{
    verify_program, CodeOffset as CO, DescriptorId, FrameOffset as FO, Function, MicroOp,
    ObjectDescriptor,
};

fn trivial_descriptors() -> Vec<ObjectDescriptor> {
    vec![ObjectDescriptor::Trivial]
}

/// A minimal well-formed function: one `Return`, args_and_locals_size 8.
fn minimal_func() -> Function {
    Function {
        code: vec![MicroOp::Return],
        args_size: 0,
        args_and_locals_size: 8,
        extended_frame_size: 32,
        zero_frame: false,
        pointer_offsets: vec![],
    }
}

// ---------------------------------------------------------------------------
// Positive: well-formed programs pass cleanly
// ---------------------------------------------------------------------------

#[test]
fn valid_minimal() {
    let errors = verify_program(&[minimal_func()], &trivial_descriptors());
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
        code,
        args_size: 0,
        args_and_locals_size: 16,
        extended_frame_size: 40,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
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
        VecPushBack { vec_ref: FO(16), elem: FO(8), elem_size: 8, descriptor_id: DescriptorId(0) },
        Return,
    ];

    let func = Function {
        code,
        args_size: 0,
        args_and_locals_size: 32,
        extended_frame_size: 56,
        zero_frame: true,
        pointer_offsets: vec![FO(0)],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

// ---------------------------------------------------------------------------
// Frame bounds violations
// ---------------------------------------------------------------------------

#[test]
fn frame_bounds_store_u64() {
    use MicroOp::*;
    let func = Function {
        code: vec![StoreImm8 { dst: FO(8), imm: 0 }, Return],
        args_and_locals_size: 8,
        extended_frame_size: 32, // offset 8 lands in metadata [8, 32)
        args_size: 0,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
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
        code: vec![
            Move {
                dst: FO(8),
                src: FO(0),
                size: 16,
            },
            Return,
        ],
        args_and_locals_size: 16,
        extended_frame_size: 40, // dst [8, 24) overlaps metadata [16, 40)
        args_size: 0,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn frame_bounds_fat_ptr_write() {
    use MicroOp::*;
    let func = Function {
        code: vec![
            StoreImm8 { dst: FO(0), imm: 0 },
            SlotBorrow {
                dst: FO(8),
                local: FO(0),
            },
            Return,
        ],
        args_and_locals_size: 16,
        extended_frame_size: 40, // dst [8, 24) overlaps metadata [16, 40)
        args_size: 0,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn frame_bounds_callfunc_metadata() {
    use MicroOp::*;
    let callee = minimal_func();
    let func = Function {
        code: vec![CallFunc { func_id: 1 }, Return],
        args_and_locals_size: 8,
        extended_frame_size: 16, // args_and_locals_size 8 + 24 = 32 > 16
        args_size: 0,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func, callee], &trivial_descriptors());
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
        code: vec![MicroOp::Return],
        args_size: 0,
        args_and_locals_size: 8,
        extended_frame_size: 32,
        zero_frame: true,
        pointer_offsets: vec![FO(100)], // offset 100 + 8 > extended_frame_size 32
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("exceeds extended_frame_size")));
}

#[test]
fn pointer_slots_overlaps_metadata() {
    let func = Function {
        code: vec![MicroOp::Return],
        args_size: 0,
        args_and_locals_size: 8,
        extended_frame_size: 40,
        zero_frame: true,
        pointer_offsets: vec![FO(8)], // offset 8 overlaps metadata [8, 32) since args_and_locals_size = 8
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn args_size_exceeds_data_size() {
    let func = Function {
        code: vec![MicroOp::Return],
        args_and_locals_size: 8,
        extended_frame_size: 32,
        args_size: 16, // > args_and_locals_size 8
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("args_size")));
}

// ---------------------------------------------------------------------------
// Jump target out of bounds
// ---------------------------------------------------------------------------

#[test]
fn invalid_jump_target() {
    use MicroOp::*;
    let func = Function {
        code: vec![
            Jump { target: CO(5) }, // only 2 instructions -> 5 >= 2
            Return,
        ],
        args_size: 0,
        args_and_locals_size: 8,
        extended_frame_size: 32,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("jump target")));
}

#[test]
fn invalid_conditional_jump_target() {
    use MicroOp::*;
    let func = Function {
        code: vec![
            StoreImm8 { dst: FO(0), imm: 0 },
            JumpNotZeroU64 {
                target: CO(99),
                src: FO(0),
            },
            Return,
        ],
        args_size: 0,
        args_and_locals_size: 8,
        extended_frame_size: 32,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("jump target")));
}

// ---------------------------------------------------------------------------
// Invalid cross-function references
// ---------------------------------------------------------------------------

#[test]
fn invalid_callfunc_func_id() {
    use MicroOp::*;
    let func = Function {
        code: vec![CallFunc { func_id: 42 }, Return],
        args_size: 0,
        args_and_locals_size: 0,
        extended_frame_size: 32,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
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
        code: vec![
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
        ],
        args_size: 0,
        args_and_locals_size: 32,
        extended_frame_size: 56,
        zero_frame: true,
        pointer_offsets: vec![FO(0)],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
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
        code: vec![
            Move {
                dst: FO(0),
                src: FO(0),
                size: 0,
            },
            Return,
        ],
        args_size: 0,
        args_and_locals_size: 8,
        extended_frame_size: 32,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("size")));
}

#[test]
fn zero_elem_size_vec_push() {
    use MicroOp::*;

    let func = Function {
        code: vec![
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
        ],
        args_size: 0,
        args_and_locals_size: 32,
        extended_frame_size: 56,
        zero_frame: true,
        pointer_offsets: vec![FO(0)],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("size")));
}

// ---------------------------------------------------------------------------
// Function-level sanity
// ---------------------------------------------------------------------------

#[test]
fn empty_code() {
    let func = Function {
        code: vec![],
        args_size: 0,
        args_and_locals_size: 8,
        extended_frame_size: 32,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("non-empty")));
}

#[test]
fn zero_frame_size() {
    let func = Function {
        code: vec![MicroOp::Return],
        args_size: 0,
        args_and_locals_size: 0,
        extended_frame_size: 0,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("frame_size")));
}

// ---------------------------------------------------------------------------
// Multiple errors collected
// ---------------------------------------------------------------------------

#[test]
fn multiple_errors_collected() {
    use MicroOp::*;
    let func = Function {
        code: vec![
            StoreImm8 {
                dst: FO(100),
                imm: 0,
            }, // out of bounds
            Jump { target: CO(99) }, // invalid target
            Return,
        ],
        args_size: 0,
        args_and_locals_size: 8,
        extended_frame_size: 32,
        zero_frame: false,
        pointer_offsets: vec![],
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(
        errors.len() >= 2,
        "expected at least 2 errors, got {}",
        errors.len()
    );
}
