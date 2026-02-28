// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the static verifier (`verify_program`).

use aptos_experimental_new_interpreter::{
    verifier::verify_program, Function, Instruction, ObjectDescriptor,
};
use std::collections::HashMap;

fn trivial_descriptors() -> Vec<ObjectDescriptor> {
    vec![ObjectDescriptor::Trivial]
}

/// A minimal well-formed function: one `Return`, data_size 8.
fn minimal_func() -> Function {
    Function {
        code: vec![Instruction::Return],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: HashMap::new(),
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
    use Instruction::*;

    #[rustfmt::skip]
    let code = vec![
        StoreU64 { dst_fp_offset: 0, val: 10 },
        StoreU64 { dst_fp_offset: 8, val: 1 },
        SubU64Const { src_fp_offset: 0, val: 1, dst_fp_offset: 0 },
        JumpIfNotZero { src_fp_offset: 0, dst_pc: 2 },
        Return,
    ];

    let func = Function {
        code,
        data_size: 16,
        extended_frame_size: 40,
        stack_maps: HashMap::new(),
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

#[test]
fn valid_with_vec_and_stack_maps() {
    use Instruction::*;

    #[rustfmt::skip]
    let code = vec![
        VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: 0 },
        StoreU64 { dst_fp_offset: 8, val: 42 },
        VecPushBack { vec_fp_offset: 0, elem_fp_offset: 8, elem_size: 8 },
        Return,
    ];

    let mut sm = HashMap::new();
    sm.insert(0, vec![]);
    sm.insert(2, vec![0]);

    let func = Function {
        code,
        data_size: 16,
        extended_frame_size: 40,
        stack_maps: sm,
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

// ---------------------------------------------------------------------------
// Frame bounds violations
// ---------------------------------------------------------------------------

#[test]
fn frame_bounds_store_u64() {
    use Instruction::*;
    let func = Function {
        code: vec![
            StoreU64 { dst_fp_offset: 8, val: 0 },
            Return,
        ],
        data_size: 8,
        extended_frame_size: 32, // offset 8 lands in metadata [8, 32)
        stack_maps: HashMap::new(),
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].message.contains("dst access"), "{}", errors[0]);
}

#[test]
fn frame_bounds_mov() {
    use Instruction::*;
    let func = Function {
        code: vec![
            Mov { src_fp_offset: 0, dst_fp_offset: 8, size: 16 },
            Return,
        ],
        data_size: 16,
        extended_frame_size: 40, // dst [8, 24) overlaps metadata [16, 40)
        stack_maps: HashMap::new(),
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("dst access")));
}

#[test]
fn frame_bounds_fat_ptr_write() {
    use Instruction::*;
    let func = Function {
        code: vec![
            StoreU64 { dst_fp_offset: 0, val: 0 },
            BorrowLocal { local_fp_offset: 0, dst_fp_offset: 8 },
            Return,
        ],
        data_size: 16,
        extended_frame_size: 40, // dst [8, 24) overlaps metadata [16, 40)
        stack_maps: HashMap::new(),
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("fat ptr")));
}

#[test]
fn frame_bounds_callfunc_metadata() {
    use Instruction::*;
    let callee = minimal_func();
    let func = Function {
        code: vec![
            CallFunc { func_id: 1 },
            Return,
        ],
        data_size: 8,
        extended_frame_size: 16, // data_size 8 + 24 = 32 > 16
        stack_maps: {
            let mut sm = HashMap::new();
            sm.insert(1, vec![]);
            sm
        },
    };
    let errors = verify_program(&[func, callee], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("extended_frame_size") && e.message.contains("frame_size()")));
}

// ---------------------------------------------------------------------------
// Missing stack maps at GC safe points
// ---------------------------------------------------------------------------

#[test]
fn missing_stack_map_vec_new() {
    use Instruction::*;
    let func = Function {
        code: vec![
            VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: 0 },
            Return,
        ],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: HashMap::new(), // missing at PC 0
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("stack map")));
}

#[test]
fn missing_stack_map_vec_push_back() {
    use Instruction::*;

    let mut sm = HashMap::new();
    sm.insert(0, vec![]);

    let func = Function {
        code: vec![
            VecNew { descriptor_id: 0, elem_size: 8, initial_capacity: 4, dst_fp_offset: 0 },
            StoreU64 { dst_fp_offset: 8, val: 1 },
            VecPushBack { vec_fp_offset: 0, elem_fp_offset: 8, elem_size: 8 },
            Return,
        ],
        data_size: 16,
        extended_frame_size: 40,
        stack_maps: sm, // missing at PC 2
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.pc == Some(2) && e.message.contains("stack map")));
}

#[test]
fn missing_stack_map_force_gc() {
    use Instruction::*;
    let func = Function {
        code: vec![
            ForceGC,
            Return,
        ],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: HashMap::new(), // missing at PC 0
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("stack map")));
}

#[test]
fn missing_stack_map_callfunc_return_site() {
    use Instruction::*;
    let callee = minimal_func();
    let func = Function {
        code: vec![
            CallFunc { func_id: 1 },
            Return,
        ],
        data_size: 0,
        extended_frame_size: 32, // data_size 0 + 24 = 24 ≤ 32
        stack_maps: HashMap::new(), // missing return-site map at PC 1
    };
    let errors = verify_program(&[func, callee], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("return site")));
}

// ---------------------------------------------------------------------------
// Jump target out of bounds
// ---------------------------------------------------------------------------

#[test]
fn invalid_jump_target() {
    use Instruction::*;
    let func = Function {
        code: vec![
            Jump { dst_pc: 5 }, // only 2 instructions → 5 ≥ 2
            Return,
        ],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: HashMap::new(),
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("jump target")));
}

#[test]
fn invalid_conditional_jump_target() {
    use Instruction::*;
    let func = Function {
        code: vec![
            StoreU64 { dst_fp_offset: 0, val: 0 },
            JumpIfNotZero { src_fp_offset: 0, dst_pc: 99 },
            Return,
        ],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: HashMap::new(),
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
    use Instruction::*;
    let func = Function {
        code: vec![
            CallFunc { func_id: 42 },
            Return,
        ],
        data_size: 0,
        extended_frame_size: 32,
        stack_maps: {
            let mut sm = HashMap::new();
            sm.insert(1, vec![]);
            sm
        },
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
    use Instruction::*;

    let mut sm = HashMap::new();
    sm.insert(0, vec![]);

    let func = Function {
        code: vec![
            VecNew { descriptor_id: 99, elem_size: 8, initial_capacity: 4, dst_fp_offset: 0 },
            Return,
        ],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: sm,
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("descriptor_id")));
}

// ---------------------------------------------------------------------------
// Stack map validity
// ---------------------------------------------------------------------------

#[test]
fn stack_map_offset_out_of_bounds() {
    use Instruction::*;

    let mut sm = HashMap::new();
    sm.insert(0, vec![100]); // offset 100 + 8 > extended_frame_size 32

    let func = Function {
        code: vec![
            ForceGC,
            Return,
        ],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: sm,
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("stack map offset")));
}

#[test]
fn stack_map_pc_out_of_bounds() {
    use Instruction::*;

    let mut sm = HashMap::new();
    sm.insert(99, vec![]); // PC 99 but only 1 instruction

    let func = Function {
        code: vec![Return],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: sm,
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("stack map PC")));
}

// ---------------------------------------------------------------------------
// Nonzero size checks
// ---------------------------------------------------------------------------

#[test]
fn zero_size_mov() {
    use Instruction::*;
    let func = Function {
        code: vec![
            Mov { src_fp_offset: 0, dst_fp_offset: 0, size: 0 },
            Return,
        ],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: HashMap::new(),
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("size")));
}

#[test]
fn zero_elem_size_vec_new() {
    use Instruction::*;

    let mut sm = HashMap::new();
    sm.insert(0, vec![]);

    let func = Function {
        code: vec![
            VecNew { descriptor_id: 0, elem_size: 0, initial_capacity: 4, dst_fp_offset: 0 },
            Return,
        ],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: sm,
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
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: HashMap::new(),
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("non-empty")));
}

#[test]
fn zero_frame_size() {
    let func = Function {
        code: vec![Instruction::Return],
        data_size: 0,
        extended_frame_size: 0,
        stack_maps: HashMap::new(),
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
    use Instruction::*;
    let func = Function {
        code: vec![
            StoreU64 { dst_fp_offset: 100, val: 0 }, // out of bounds
            Jump { dst_pc: 99 },                       // invalid target
            Return,
        ],
        data_size: 8,
        extended_frame_size: 32,
        stack_maps: HashMap::new(),
    };
    let errors = verify_program(&[func], &trivial_descriptors());
    assert!(errors.len() >= 2, "expected at least 2 errors, got {}", errors.len());
}
