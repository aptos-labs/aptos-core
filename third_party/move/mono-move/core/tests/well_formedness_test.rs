// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Tests for the well-formedness checker (`check_well_formedness`).

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    check_well_formedness,
    interner::{FunctionRef, InternedModuleId, ModuleId},
    native::{FrameSlot, NativeABI, NativeIdx},
    types::{
        InternedType, InternedTypeList, Type, ADDRESS_TY, EMPTY_TYPE_LIST, U32_TY, U64_TY, U8_TY,
    },
    CallClosureOp, ClosureFuncRef, Code, CodeOffset as CO, ConstantPoolIndex, DescriptorId,
    DescriptorProvider, FrameLayoutInfo, FrameOffset as FO, Function, FunctionDefinitionIndex,
    FunctionPtr, IntBinaryOp, IntNegateOp, IntOperand, IntShiftOp, IntTy, JumpValueCmpOp,
    JumpValueRefCmpOp, LayoutId, LayoutProvider, MicroOp, ObjectDescriptor, ObjectDescriptorTable,
    PackClosureOp, SafePointEntry, ShiftOperand, SizedSlot, SortedSafePointEntries, ValueCmpOp,
    ValueLayout, ValueLayoutTable, ValueRefCmpOp, VecUnpackOp, POINTER_VEC_DESCRIPTOR_ID,
    TRIVIAL_DESCRIPTOR_ID,
};

/// Published layouts and descriptors, including the reserved primitive layouts.
struct TestProvider {
    descriptors: ObjectDescriptorTable,
    layouts: ValueLayoutTable,
}

impl TestProvider {
    fn new(descriptors: ObjectDescriptorTable) -> Self {
        Self {
            descriptors,
            layouts: ValueLayoutTable::new(),
        }
    }
}

/// The constant pool of a function that has no `StoreImmVec`.
const NO_CONSTANTS: &[InternedType] = &[];

impl DescriptorProvider for TestProvider {
    fn descriptor(&self, id: DescriptorId) -> Option<&ObjectDescriptor> {
        self.descriptors.descriptor(id)
    }
}

impl LayoutProvider for TestProvider {
    fn layout(&self, id: LayoutId) -> Option<&ValueLayout> {
        self.layouts.layout(id)
    }

    fn layout_id(&self, ty: InternedType) -> Option<LayoutId> {
        self.layouts.layout_id(ty)
    }
}

/// A table holding only the reserved descriptors.
fn trivial_descriptors() -> TestProvider {
    TestProvider::new(ObjectDescriptorTable::new())
}

/// Interned module id for hand-built test functions.
fn test_module_id() -> InternedModuleId {
    static MODULE_ID: ModuleId = ModuleId::new(
        move_core_types::account_address::AccountAddress::ONE,
        GlobalArenaPtr::from_static("test"),
    );
    GlobalArenaPtr::from_static(&MODULE_ID)
}

/// A minimal well-formed function: one `Return`, param_and_local_sizes_sum 8.
fn minimal_func() -> Function {
    Function {
        name: GlobalArenaPtr::from_static("test"),
        module_id: test_module_id(),
        def_idx: FunctionDefinitionIndex(0),
        code: Code::from_vec(vec![MicroOp::Return]),
        entry_gas: 0,
        param_slots: vec![],
        param_tys: vec![],
        return_slots: vec![],
        return_tys: EMPTY_TYPE_LIST,
        param_region_size: 0,
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
    assert_accepted(&func);
}

#[test]
fn valid_with_arithmetic_and_jumps() {
    use MicroOp::*;

    #[rustfmt::skip]
    let code = vec![
        StoreImm8 { dst: FO(0), imm: 10u64.to_le_bytes() },
        StoreImm8 { dst: FO(8), imm: 1u64.to_le_bytes() },
        SubU64Imm { dst: FO(0), src: FO(0), imm: 1 },
        JumpNotZeroU64 { target: CO(2), src: FO(0), gas_taken: 0, gas_fallthrough: 0 },
        Return,
    ];
    let func = Function {
        code: Code::from_vec(code),
        param_and_local_sizes_sum: 16,
        extended_frame_size: 40,
        ..minimal_func()
    };
    assert_accepted(&func);
}

#[test]
fn valid_with_vec_and_pointer_slots() {
    use MicroOp::*;

    #[rustfmt::skip]
    let code = vec![
        VecNew { dst: FO(0) },
        SlotBorrow { dst: FO(16), local: FO(0) },
        StoreImm8 { dst: FO(8), imm: 42u64.to_le_bytes() },
        VecPushBack { vec_ref: FO(16), elem: FO(8), elem_size: 8, descriptor_id: POINTER_VEC_DESCRIPTOR_ID },
        Return,
    ];
    let func = Function {
        code: Code::from_vec(code),
        param_and_local_sizes_sum: 32,
        extended_frame_size: 56,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..minimal_func()
    };
    assert_accepted(&func);
}

// ---------------------------------------------------------------------------
// Frame bounds violations
// ---------------------------------------------------------------------------

#[test]
fn frame_bounds_store_u64() {
    use MicroOp::*;
    let func = Function {
        code: Code::from_vec(vec![
            StoreImm8 {
                dst: FO(8),
                imm: 0u64.to_le_bytes(),
            },
            Return,
        ]),
        extended_frame_size: 32, // offset 8 lands in metadata [8, 32)
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
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
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn frame_bounds_fat_ptr_write() {
    use MicroOp::*;
    let func = Function {
        code: Code::from_vec(vec![
            StoreImm8 {
                dst: FO(0),
                imm: 0u64.to_le_bytes(),
            },
            SlotBorrow {
                dst: FO(8),
                local: FO(0),
            },
            Return,
        ]),
        param_and_local_sizes_sum: 16,
        extended_frame_size: 40, // dst [8, 24) overlaps metadata [16, 40)
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn frame_bounds_extended_frame_too_small() {
    let func = Function {
        extended_frame_size: 16, // param_and_local_sizes_sum 8 + 24 = 32 > 16
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("extended_frame_size") && e.message.contains("frame_size()")));
}

#[test]
fn origins_table_must_be_empty_or_match_code_length() {
    // Two-entry origins table against one-op code.
    let mut func = minimal_func();
    func.code = Code::with_origins(vec![MicroOp::Return], vec![0, 0]);
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(errors
        .iter()
        .any(|error| error.message.contains("number of origins")));

    // A table with one entry per micro-op passes.
    func.code = Code::with_origins(vec![MicroOp::Return], vec![0]);
    assert_accepted(&func);
}

// ---------------------------------------------------------------------------
// Pointer slots validation
// ---------------------------------------------------------------------------

#[test]
fn pointer_slots_offset_out_of_bounds() {
    let func = Function {
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(100)]), // offset 100 + 8 > extended_frame_size 32
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("exceeds extended_frame_size")));
}

#[test]
fn pointer_slots_overlaps_metadata() {
    let func = Function {
        extended_frame_size: 40,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(8)]), // offset 8 overlaps metadata [8, 32) since param_and_local_sizes_sum = 8
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("overlaps metadata")));
}

#[test]
fn param_and_local_sizes_sum_misaligned() {
    // SAFETY: Arena is alive for the duration of the test.
    let func = Function {
        param_and_local_sizes_sum: 1, // not a multiple of 8
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("8-byte aligned")));
}

#[test]
fn args_size_exceeds_data_size() {
    let func = Function {
        param_region_size: 16, // > param_and_local_sizes_sum 8
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors
        .iter()
        .any(|e| e.message.contains("param_region_size")));
}

// ---------------------------------------------------------------------------
// Jump target out of bounds
// ---------------------------------------------------------------------------

#[test]
fn invalid_jump_target() {
    use MicroOp::*;
    let func = Function {
        code: Code::from_vec(vec![
            Jump {
                target: CO(5),
                gas: 0,
            }, // only 2 instructions -> 5 >= 2
            Return,
        ]),
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("jump target")));
}

#[test]
fn invalid_conditional_jump_target() {
    use MicroOp::*;
    let func = Function {
        code: Code::from_vec(vec![
            StoreImm8 {
                dst: FO(0),
                imm: 0u64.to_le_bytes(),
            },
            JumpNotZeroU64 {
                target: CO(99),
                src: FO(0),
                gas_taken: 0,
                gas_fallthrough: 0,
            },
            Return,
        ]),
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("jump target")));
}

// ---------------------------------------------------------------------------
// Invalid descriptor ID
// ---------------------------------------------------------------------------

#[test]
fn invalid_descriptor_id() {
    use MicroOp::*;

    let func = Function {
        code: Code::from_vec(vec![
            VecNew { dst: FO(0) },
            SlotBorrow {
                dst: FO(8),
                local: FO(0),
            },
            StoreImm8 {
                dst: FO(24),
                imm: 42u64.to_le_bytes(),
            },
            VecPushBack {
                vec_ref: FO(8),
                elem: FO(24),
                elem_size: 8,
                descriptor_id: DescriptorId(99),
            },
            Return,
        ]),
        param_and_local_sizes_sum: 32,
        extended_frame_size: 56,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
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
        code: Code::from_vec(vec![
            Move {
                dst: FO(0),
                src: FO(0),
                size: 0,
            },
            Return,
        ]),
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("size")));
}

#[test]
fn zero_elem_size_vec_push() {
    use MicroOp::*;

    let func = Function {
        code: Code::from_vec(vec![
            VecNew { dst: FO(0) },
            SlotBorrow {
                dst: FO(8),
                local: FO(0),
            },
            StoreImm8 {
                dst: FO(24),
                imm: 42u64.to_le_bytes(),
            },
            VecPushBack {
                vec_ref: FO(8),
                elem: FO(24),
                elem_size: 0,
                descriptor_id: TRIVIAL_DESCRIPTOR_ID,
            },
            Return,
        ]),
        param_and_local_sizes_sum: 32,
        extended_frame_size: 56,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("size")));
}

// ---------------------------------------------------------------------------
// Function-level sanity
// ---------------------------------------------------------------------------

#[test]
fn empty_code() {
    let func = Function {
        code: Code::from_vec(vec![]),
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("non-empty")));
}

#[test]
fn zero_frame_size() {
    let func = Function {
        param_and_local_sizes_sum: 0,
        extended_frame_size: 0,
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(!errors.is_empty());
    assert!(errors.iter().any(|e| e.message.contains("frame_size")));
}

// ---------------------------------------------------------------------------
// Static arithmetic constraints (imm-form ops)
// ---------------------------------------------------------------------------
//
// Unchecked u64 division, remainder, and shift ops require nonzero divisors
// and shift amounts below 64. The checker rejects violations of these
// lowering invariants.

fn func_with_single_op(op: MicroOp) -> Function {
    Function {
        code: Code::from_vec(vec![op, MicroOp::Return]),
        param_and_local_sizes_sum: 24,
        extended_frame_size: 48,
        ..minimal_func()
    }
}

#[test]
fn div_u64_imm_zero() {
    let func = func_with_single_op(MicroOp::DivU64Imm {
        dst: FO(0),
        src: FO(8),
        imm: 0,
    });
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
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
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
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
    assert_accepted(&func);
}

#[test]
fn shl_u64_imm_oversize() {
    let func = func_with_single_op(MicroOp::ShlU64Imm {
        dst: FO(0),
        src: FO(8),
        imm: 64,
    });
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
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
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
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
    assert_accepted(&func);
}

// ---------------------------------------------------------------------------
// Multiple errors collected
// ---------------------------------------------------------------------------

#[test]
fn multiple_errors_collected() {
    use MicroOp::*;
    let func = Function {
        code: Code::from_vec(vec![
            StoreImm8 {
                dst: FO(100),
                imm: 0u64.to_le_bytes(),
            }, // out of bounds
            Jump {
                target: CO(99),
                gas: 0,
            }, // invalid target
            Return,
        ]),
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
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

fn vec_pushback_func(descriptor_id: DescriptorId) -> Function {
    use MicroOp::*;
    Function {
        code: Code::from_vec(vec![
            VecNew { dst: FO(0) },
            SlotBorrow {
                dst: FO(16),
                local: FO(0),
            },
            StoreImm8 {
                dst: FO(8),
                imm: 1u64.to_le_bytes(),
            },
            VecPushBack {
                vec_ref: FO(16),
                elem: FO(8),
                elem_size: 8,
                descriptor_id,
            },
            Return,
        ]),
        param_and_local_sizes_sum: 32,
        extended_frame_size: 56,
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..minimal_func()
    }
}

#[test]
fn vec_pushback_accepts_trivial_descriptor() {
    // A pointer-free vector canonically uses the Trivial descriptor.
    let func = vec_pushback_func(TRIVIAL_DESCRIPTOR_ID);
    assert_accepted(&func);
}

#[test]
fn vec_pushback_rejects_non_vector_descriptor() {
    // A Struct descriptor is neither Trivial nor a Vector.
    let mut descriptors = ObjectDescriptorTable::new();
    let struct_desc = descriptors.push(ObjectDescriptor::new_struct(8, vec![]).unwrap());
    let func = vec_pushback_func(struct_desc);
    let errors = check_well_formedness(&func, &TestProvider::new(descriptors), NO_CONSTANTS);
    assert!(errors.iter().any(|e| e.message.contains("VecPushBack")
        && e.message.contains("not a non-empty Vector or Trivial")));
}

#[test]
fn heap_new_rejects_vector_descriptor() {
    use MicroOp::*;
    let func = Function {
        code: Code::from_vec(vec![
            HeapNew {
                dst: FO(0),
                descriptor_id: POINTER_VEC_DESCRIPTOR_ID,
            },
            Return,
        ]),
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..minimal_func()
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("not a Struct or Enum")));
}

// ---------------------------------------------------------------------------
// Helpers for the checks below
// ---------------------------------------------------------------------------

fn slot(offset: u32, size: u32, align: u32) -> SizedSlot {
    SizedSlot {
        offset: FO(offset),
        size,
        align,
    }
}

/// `[T]` type list with a single `u64`.
fn one_u64() -> InternedTypeList {
    static LIST: [InternedType; 1] = [U64_TY];
    let list: &'static [InternedType] = &LIST;
    InternedTypeList::new(GlobalArenaPtr::from_static(list))
}

/// `[T]` type list with two `u64`s.
fn two_u64() -> InternedTypeList {
    static LIST: [InternedType; 2] = [U64_TY, U64_TY];
    let list: &'static [InternedType] = &LIST;
    InternedTypeList::new(GlobalArenaPtr::from_static(list))
}

fn ref_u64() -> InternedType {
    static REF: Type = Type::ImmutRef { inner: U64_TY };
    GlobalArenaPtr::from_static(&REF)
}

fn errors_of(func: &Function) -> Vec<String> {
    check_well_formedness(func, &trivial_descriptors(), NO_CONSTANTS)
        .into_iter()
        .map(|e| e.message)
        .collect()
}

fn assert_error_contains(func: &Function, needle: &str) {
    let errors = errors_of(func);
    assert!(
        errors.iter().any(|m| m.contains(needle)),
        "expected an error containing {needle:?}, got {errors:#?}"
    );
}

fn assert_accepted(func: &Function) {
    let errors = errors_of(func);
    assert!(errors.is_empty(), "unexpected errors: {errors:#?}");
}

/// A callee with one `u64` parameter at offset 0 and no return values.
fn one_param_callee() -> Function {
    Function {
        param_slots: vec![slot(0, 8, 8)],
        param_tys: vec![U64_TY],
        param_region_size: 8,
        ..minimal_func()
    }
}

// ---------------------------------------------------------------------------
// Alignment
// ---------------------------------------------------------------------------

#[test]
fn misaligned_u64_slot_rejected() {
    let func = func_with_single_op(MicroOp::AddU64 {
        dst: FO(4),
        lhs: FO(8),
        rhs: FO(8),
    });
    assert_error_contains(&func, "not 8-byte aligned");
}

#[test]
fn misaligned_fat_pointer_rejected() {
    let func = func_with_single_op(MicroOp::VecLen {
        dst: FO(0),
        vec_ref: FO(4),
    });
    assert_error_contains(&func, "[4, 20) is not 8-byte aligned");
}

#[test]
fn misaligned_pointer_offset_rejected() {
    let func = Function {
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(4)]),
        param_and_local_sizes_sum: 24,
        extended_frame_size: 48,
        ..minimal_func()
    };
    assert_error_contains(&func, "[4, 12) is not 8-byte aligned");
}

#[test]
fn move8_and_byte_copies_need_no_alignment() {
    assert_accepted(&func_with_single_op(MicroOp::Move8 {
        dst: FO(1),
        src: FO(9),
    }));
    assert_accepted(&func_with_single_op(MicroOp::Move {
        dst: FO(1),
        src: FO(9),
        size: 8,
    }));
    // The frame side of the 8-byte heap moves is unaligned too.
    assert_accepted(&func_with_single_op(MicroOp::HeapMoveFrom8 {
        dst: FO(1),
        heap_ptr: FO(16),
        offset: 0,
    }));
}

#[test]
fn int_slots_use_natural_alignment_up_to_max_align() {
    // u32 operands must be 4-aligned.
    let func = func_with_single_op(MicroOp::IntAdd(IntBinaryOp {
        dst: FO(2),
        lhs: FO(0),
        rhs: IntOperand::SlotU32(FO(4)),
    }));
    assert_error_contains(&func, "[2, 6) is not 4-byte aligned");
    // u128 slots are 8-aligned under the layout convention, even though the
    // interpreter happens to read them unaligned.
    let func = func_with_single_op(MicroOp::IntAdd(IntBinaryOp {
        dst: FO(4),
        lhs: FO(4),
        rhs: IntOperand::SlotU128(FO(4)),
    }));
    assert_error_contains(&func, "[4, 20) is not 8-byte aligned");
    assert_accepted(&func_with_single_op(MicroOp::IntAdd(IntBinaryOp {
        dst: FO(8),
        lhs: FO(8),
        rhs: IntOperand::SlotU128(FO(8)),
    })));
}

#[test]
fn value_cmp_uses_the_layout_alignment() {
    // A vector compares through an aligned 8-byte pointer.
    static VEC_U64: Type = Type::Vector { elem: U64_TY };
    let vec_ty: InternedType = GlobalArenaPtr::from_static(&VEC_U64);
    let func = func_with_single_op(MicroOp::ValueCmp(ValueCmpOp {
        negate: false,
        dst: FO(0),
        lhs: FO(4),
        rhs: FO(16),
        ty: vec_ty,
    }));
    let mut provider = trivial_descriptors();
    let elem_id = provider.layout_id(U64_TY).unwrap();
    provider
        .layouts
        .push(ValueLayout::vector(vec_ty, elem_id, TRIVIAL_DESCRIPTOR_ID));
    let errors = check_well_formedness(&func, &provider, NO_CONSTANTS);
    assert!(
        errors
            .iter()
            .any(|e| e.message.contains("[4, 12) is not 8-byte aligned")),
        "{errors:#?}"
    );
}

// ---------------------------------------------------------------------------
// Frame geometry, parameter and return slots
// ---------------------------------------------------------------------------

#[test]
fn callee_fp_must_be_aligned() {
    // `param_and_local_sizes_sum` is 8-aligned, but `frame_size()` is what
    // the callee fp lands on; with FRAME_METADATA_SIZE = 24 the two agree,
    // so only the first message fires for a misaligned sum.
    let func = Function {
        param_and_local_sizes_sum: 12,
        extended_frame_size: 48,
        ..minimal_func()
    };
    assert_error_contains(
        &func,
        "param_and_local_sizes_sum (12) must be 8-byte aligned",
    );
    assert_error_contains(&func, "frame_size() (36) must be 8-byte aligned");
}

#[test]
fn param_slot_count_must_match_param_types() {
    let func = Function {
        param_slots: vec![slot(0, 8, 8)],
        param_tys: vec![],
        param_region_size: 8,
        ..minimal_func()
    };
    assert_error_contains(
        &func,
        "number of param slots (1) must equal number of param types (0)",
    );
}

#[test]
fn param_slot_must_lie_in_param_region() {
    let func = Function {
        param_region_size: 0,
        ..one_param_callee()
    };
    assert_error_contains(&func, "param slot [0, 8) exceeds param_region_size (0)");
    assert_accepted(&one_param_callee());
}

#[test]
fn param_slots_must_be_ascending_and_disjoint() {
    let func = Function {
        param_slots: vec![slot(0, 8, 8), slot(4, 8, 4)],
        param_tys: vec![U64_TY, U64_TY],
        param_region_size: 16,
        param_and_local_sizes_sum: 16,
        extended_frame_size: 40,
        ..minimal_func()
    };
    assert_error_contains(&func, "param slot 1: offset 4 != signature layout offset 8");
}

#[test]
fn sized_slot_alignment_is_validated() {
    let bad_align = Function {
        param_slots: vec![slot(0, 8, 0)],
        ..one_param_callee()
    };
    assert_error_contains(
        &bad_align,
        "param slot 0: align 0 must be a power of two in [1, 8]",
    );
    let too_big = Function {
        param_slots: vec![slot(0, 8, 16)],
        ..one_param_callee()
    };
    assert_error_contains(&too_big, "align 16 must be a power of two in [1, 8]");
    let misaligned = Function {
        param_slots: vec![slot(4, 8, 8)],
        param_region_size: 16,
        param_and_local_sizes_sum: 16,
        extended_frame_size: 40,
        ..one_param_callee()
    };
    assert_error_contains(&misaligned, "param slot 0: offset 4 is not 8-byte aligned");
    let zero_size = Function {
        param_slots: vec![slot(0, 0, 8)],
        ..one_param_callee()
    };
    assert_error_contains(&zero_size, "param slot 0: size must be > 0");
}

#[test]
fn return_slot_count_must_match_return_types() {
    let func = Function {
        return_slots: vec![slot(0, 8, 8)],
        return_tys: EMPTY_TYPE_LIST,
        ..minimal_func()
    };
    assert_error_contains(
        &func,
        "number of return slots (1) must equal number of return types (0)",
    );
}

#[test]
fn return_slots_must_fit_data_region_and_be_disjoint() {
    let too_wide = Function {
        return_slots: vec![slot(0, 16, 8)],
        return_tys: one_u64(),
        ..minimal_func()
    };
    assert_error_contains(
        &too_wide,
        "return slot [0, 16) exceeds param_and_local_sizes_sum (8)",
    );
    let overlapping = Function {
        return_slots: vec![slot(0, 8, 8), slot(0, 8, 8)],
        return_tys: two_u64(),
        param_and_local_sizes_sum: 16,
        extended_frame_size: 40,
        ..minimal_func()
    };
    assert_error_contains(
        &overlapping,
        "return slot 1: offset 0 != signature layout offset 8",
    );
}

// ---------------------------------------------------------------------------
// GC layouts
// ---------------------------------------------------------------------------

#[test]
fn pointer_slot_beyond_params_requires_zero_frame() {
    let func = Function {
        zero_frame: false,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..minimal_func()
    };
    assert_error_contains(
        &func,
        "pointer slot 0 beyond param_region_size (0) but zero_frame is false",
    );
    assert_accepted(&Function {
        zero_frame: true,
        ..func
    });
    // A pointer-typed parameter is written by the caller and needs no zeroing.
    assert_accepted(&Function {
        zero_frame: false,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..one_param_callee()
    });
}

#[test]
fn safe_points_must_be_sorted_in_bounds_at_allocating_ops_and_disjoint_from_base() {
    let safe_point = |co: u32, offsets: Vec<FO>| SafePointEntry {
        code_offset: CO(co),
        layout: FrameLayoutInfo::new(offsets),
    };
    // `ForceGC` is allocating; `Return` is not.
    let base = || Function {
        code: Code::from_vec(vec![MicroOp::ForceGC, MicroOp::Return]),
        param_and_local_sizes_sum: 24,
        extended_frame_size: 48,
        ..minimal_func()
    };
    let unsorted = Function {
        safe_point_layouts: SortedSafePointEntries::new(vec![
            safe_point(1, vec![]),
            safe_point(0, vec![]),
        ]),
        ..base()
    };
    assert_error_contains(&unsorted, "entries not strictly sorted");
    let out_of_bounds = Function {
        safe_point_layouts: SortedSafePointEntries::new(vec![safe_point(5, vec![])]),
        ..base()
    };
    assert_error_contains(&out_of_bounds, "code_offset 5 out of bounds");
    let not_allocating = Function {
        safe_point_layouts: SortedSafePointEntries::new(vec![safe_point(1, vec![FO(0)])]),
        ..base()
    };
    assert_error_contains(&not_allocating, "is not at an allocating op");
    let duplicate = Function {
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        safe_point_layouts: SortedSafePointEntries::new(vec![safe_point(0, vec![FO(0)])]),
        ..base()
    };
    assert_error_contains(&duplicate, "offset 0 duplicates frame_layout");
    let ok = Function {
        safe_point_layouts: SortedSafePointEntries::new(vec![safe_point(0, vec![FO(8)])]),
        ..base()
    };
    assert_accepted(&ok);
}

// ---------------------------------------------------------------------------
// Control flow
// ---------------------------------------------------------------------------

#[test]
fn last_op_must_be_a_terminator() {
    let falls_off = Function {
        code: Code::from_vec(vec![MicroOp::StoreImm8 {
            dst: FO(0),
            imm: [0; 8],
        }]),
        ..minimal_func()
    };
    assert_error_contains(&falls_off, "last op must be a terminator");
    for terminator in [
        MicroOp::Return,
        MicroOp::Abort { code: FO(0) },
        MicroOp::Jump {
            target: CO(0),
            gas: 0,
        },
    ] {
        assert_accepted(&Function {
            code: Code::from_vec(vec![terminator]),
            ..minimal_func()
        });
    }
}

// ---------------------------------------------------------------------------
// Statically decidable runtime invariant violations
// ---------------------------------------------------------------------------

#[test]
fn scalar_operands_must_not_alias_pointer_slots() {
    let with_ptr_slot = |op| Function {
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..func_with_single_op(op)
    };
    // A u64 op on the pointer slot itself.
    assert_error_contains(
        &with_ptr_slot(MicroOp::AddU64 {
            dst: FO(0),
            lhs: FO(8),
            rhs: FO(8),
        }),
        "access [0, 8) aliases frame_layout pointer slot 0",
    );
    // A u32 op straddling the pointer slot's tail.
    assert_error_contains(
        &with_ptr_slot(MicroOp::IntAdd(IntBinaryOp {
            dst: FO(4),
            lhs: FO(8),
            rhs: IntOperand::SlotU32(FO(8)),
        })),
        "access [4, 8) aliases frame_layout pointer slot 0",
    );
    // Byte copies and pointer kinds may touch it.
    assert_accepted(&with_ptr_slot(MicroOp::Move8 {
        dst: FO(0),
        src: FO(8),
    }));
    assert_accepted(&with_ptr_slot(MicroOp::VecNew { dst: FO(0) }));
    // A scalar next to the slot is fine.
    assert_accepted(&with_ptr_slot(MicroOp::StoreRandomU64 { dst: FO(8) }));
}

#[test]
fn scalar_operands_must_not_alias_safe_point_pointer_slots() {
    // `MoveFrom` allocates, so it may carry a safe point; its 32-byte `addr`
    // operand at 0 overlaps a pointer slot listed at 8.
    let func = Function {
        code: Code::from_vec(vec![
            MicroOp::MoveFrom {
                dst: FO(32),
                addr: FO(0),
                ty: U64_TY,
            },
            MicroOp::Return,
        ]),
        param_and_local_sizes_sum: 40,
        extended_frame_size: 64,
        safe_point_layouts: SortedSafePointEntries::new(vec![SafePointEntry {
            code_offset: CO(0),
            layout: FrameLayoutInfo::new(vec![FO(8)]),
        }]),
        ..minimal_func()
    };
    assert_error_contains(&func, "access [0, 32) aliases safe-point pointer slot 8");
}

#[test]
fn bitwise_on_signed_rejected() {
    let func = func_with_single_op(MicroOp::IntBitAnd(IntBinaryOp {
        dst: FO(0),
        lhs: FO(0),
        rhs: IntOperand::SlotI64(FO(8)),
    }));
    assert_error_contains(&func, "bitwise on signed type");
}

#[test]
fn shift_on_signed_rejected() {
    let func = func_with_single_op(MicroOp::IntShl(IntShiftOp {
        ty: IntTy::I64,
        dst: FO(0),
        lhs: FO(0),
        rhs: ShiftOperand::ImmU8(1),
    }));
    assert_error_contains(&func, "shift on signed type");
    assert_accepted(&func_with_single_op(MicroOp::IntShl(IntShiftOp {
        ty: IntTy::U64,
        dst: FO(0),
        lhs: FO(0),
        rhs: ShiftOperand::SlotU8(FO(8)),
    })));
}

#[test]
fn negate_on_unsigned_rejected() {
    let func = func_with_single_op(MicroOp::IntNegate(IntNegateOp {
        ty: IntTy::U64,
        dst: FO(0),
        src: FO(0),
    }));
    assert_error_contains(&func, "negate on unsigned type");
    assert_accepted(&func_with_single_op(MicroOp::IntNegate(IntNegateOp {
        ty: IntTy::I64,
        dst: FO(0),
        src: FO(0),
    })));
}

#[test]
fn value_cmp_on_reference_type_rejected() {
    let func = func_with_single_op(MicroOp::ValueCmp(ValueCmpOp {
        negate: false,
        dst: FO(0),
        lhs: FO(0),
        rhs: FO(8),
        ty: ref_u64(),
    }));
    assert_error_contains(&func, "value comparison on a reference type");
}

// ---------------------------------------------------------------------------
// Offsets, sizes, and multi-destination ops
// ---------------------------------------------------------------------------

#[test]
fn heap_offset_plus_size_must_not_overflow() {
    assert_error_contains(
        &func_with_single_op(MicroOp::HeapReadOffset {
            dst: FO(0),
            obj_ref: FO(8),
            offset: u32::MAX,
            size: 1,
        }),
        "overflows u32",
    );
    assert_error_contains(
        &func_with_single_op(MicroOp::HeapMoveFrom {
            dst: FO(0),
            heap_ptr: FO(8),
            offset: u32::MAX,
            size: 8,
        }),
        "overflows u32",
    );
    assert_error_contains(
        &func_with_single_op(MicroOp::HeapMoveToImm8 {
            heap_ptr: FO(8),
            offset: u32::MAX - 4,
            imm: 0,
        }),
        "overflows u32",
    );
    assert_error_contains(
        &func_with_single_op(MicroOp::EnumReadVariantFieldByTag {
            dst: FO(0),
            enum_ref: FO(8),
            offsets: Box::new([None, Some(u32::MAX)]),
            size: 4,
        }),
        "overflows u32",
    );
}

#[test]
fn deep_copy_heap_ptrs_offsets_are_checked() {
    assert_error_contains(
        &func_with_single_op(MicroOp::DeepCopyHeapPtrs {
            base: FO(u32::MAX),
            offsets: Box::new([1]),
        }),
        "overflows u32",
    );
    assert_error_contains(
        &func_with_single_op(MicroOp::DeepCopyHeapPtrs {
            base: FO(0),
            offsets: Box::new([4]),
        }),
        "[4, 12) is not 8-byte aligned",
    );
    assert_accepted(&func_with_single_op(MicroOp::DeepCopyHeapPtrs {
        base: FO(0),
        offsets: Box::new([0, 8]),
    }));
}

#[test]
fn vec_unpack_destinations_must_be_disjoint() {
    let func = func_with_single_op(MicroOp::VecUnpack(Box::new(VecUnpackOp {
        src: FO(16),
        elem_size: 8,
        dsts: vec![FO(0), FO(4)],
    })));
    assert_error_contains(&func, "destinations [0, 8) and [4, 12) overlap");
    assert_accepted(&func_with_single_op(MicroOp::VecUnpack(Box::new(
        VecUnpackOp {
            src: FO(16),
            elem_size: 8,
            dsts: vec![FO(8), FO(0)],
        },
    ))));
}

#[test]
fn slot_borrow_base_must_be_in_data_region() {
    let func = func_with_single_op(MicroOp::SlotBorrow {
        dst: FO(0),
        local: FO(24),
    });
    assert_error_contains(
        &func,
        "SlotBorrow local 24 is outside the data region [0, 24)",
    );
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

#[test]
fn store_imm_vec_constant_index_must_exist() {
    let func = func_with_single_op(MicroOp::StoreImmVec {
        dst: FO(0),
        idx: ConstantPoolIndex(0),
    });
    let errors = check_well_formedness(&func, &trivial_descriptors(), NO_CONSTANTS);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("constant pool index 0 out of range")));
}

#[test]
fn store_imm_vec_destination_is_sized_for_the_constant() {
    let func = func_with_single_op(MicroOp::StoreImmVec {
        dst: FO(0),
        idx: ConstantPoolIndex(0),
    });
    // A u64 constant fits an 8-byte slot...
    let errors = check_well_formedness(&func, &trivial_descriptors(), &[U64_TY][..]);
    assert!(errors.is_empty(), "{errors:#?}");
    // ...but a 32-byte address written at offset 0 runs into the metadata.
    let errors = check_well_formedness(&func, &trivial_descriptors(), &[ADDRESS_TY][..]);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("[0, 32) overlaps metadata")));
}

// ---------------------------------------------------------------------------
// Descriptors
// ---------------------------------------------------------------------------

#[test]
fn enum_new_tag_must_name_a_variant() {
    let mut descriptors = ObjectDescriptorTable::new();
    let enum_desc = descriptors.push(ObjectDescriptor::new_enum(16, vec![vec![]]).unwrap());
    let struct_desc = descriptors.push(ObjectDescriptor::new_struct(8, vec![]).unwrap());
    let provider = TestProvider::new(descriptors);
    let enum_new = |descriptor_id, variant| {
        func_with_single_op(MicroOp::EnumNew {
            dst: FO(0),
            descriptor_id,
            variant,
        })
    };
    let errors = check_well_formedness(&enum_new(enum_desc, 1), &provider, NO_CONSTANTS);
    assert!(errors
        .iter()
        .any(|e| e.message.contains("tag 1 out of range")));
    let errors = check_well_formedness(&enum_new(struct_desc, 0), &provider, NO_CONSTANTS);
    assert!(errors.iter().any(|e| e.message.contains("is not an Enum")));
    let errors = check_well_formedness(&enum_new(enum_desc, 0), &provider, NO_CONSTANTS);
    assert!(errors.is_empty(), "{errors:#?}");
}

// ---------------------------------------------------------------------------
// Calls
// ---------------------------------------------------------------------------

fn native_call(abi: NativeABI) -> MicroOp {
    MicroOp::CallNative {
        native_idx: NativeIdx(0),
        ty_args: EMPTY_TYPE_LIST,
        abi: Box::new(abi),
    }
}

fn native_abi(args: Vec<FrameSlot>, heap_ptr_offsets: Vec<FO>) -> NativeABI {
    NativeABI::new(args, vec![], heap_ptr_offsets, vec![]).unwrap()
}

#[test]
fn native_slot_region_must_fit_the_frame() {
    let abi = native_abi(vec![FrameSlot { offset: 0, size: 8 }], vec![]);
    // frame_size() is 48 here, so the native's 8-byte region ends at 56.
    assert_error_contains(
        &func_with_single_op(native_call(abi.clone())),
        "native slot region [48, 56) exceeds extended_frame_size 48",
    );
    assert_accepted(&Function {
        extended_frame_size: 56,
        ..func_with_single_op(native_call(abi))
    });
}

#[test]
fn native_pointer_offsets_are_aligned_bounded_and_inside_args() {
    let with_abi = |abi| Function {
        extended_frame_size: 72,
        ..func_with_single_op(native_call(abi))
    };
    let beyond_region = native_abi(vec![FrameSlot { offset: 0, size: 8 }], vec![FO(8)]);
    assert_error_contains(
        &with_abi(beyond_region),
        "native heap pointer offset 8 exceeds the slot region (8)",
    );
    let misaligned = native_abi(
        vec![FrameSlot {
            offset: 0,
            size: 16,
        }],
        vec![FO(4)],
    );
    assert_error_contains(
        &with_abi(misaligned),
        "native heap pointer offset 4 is not 8-byte aligned",
    );
    let outside_arg = native_abi(
        vec![FrameSlot { offset: 0, size: 8 }, FrameSlot {
            offset: 16,
            size: 8,
        }],
        vec![FO(8)],
    );
    assert_error_contains(
        &with_abi(outside_arg),
        "native heap pointer offset 8 is not inside an argument slot",
    );
    let ok = native_abi(
        vec![FrameSlot {
            offset: 0,
            size: 16,
        }],
        vec![FO(0), FO(8)],
    );
    assert_accepted(&with_abi(ok));
}

#[test]
fn native_required_descriptors_must_exist() {
    // Mint an id the trivial table does not hold.
    let mut other = ObjectDescriptorTable::new();
    let missing = other.push(ObjectDescriptor::new_struct(8, vec![]).unwrap());
    let abi = NativeABI::new(vec![], vec![], vec![], vec![missing]).unwrap();
    assert_error_contains(
        &func_with_single_op(native_call(abi)),
        "native required descriptor 0",
    );
}

#[test]
fn call_direct_callee_must_fit_the_callee_region() {
    let callee = FunctionPtr::new(Box::new(Function {
        return_slots: vec![slot(0, 8, 8)],
        return_tys: one_u64(),
        ..one_param_callee()
    }));
    let call = MicroOp::CallDirect { ptr: callee };
    // No callee region at all: frame_size() == extended_frame_size.
    assert_error_contains(
        &func_with_single_op(call.clone()),
        "CallDirect: callee param_region_size 8 exceeds the callee region (0)",
    );
    assert_error_contains(
        &func_with_single_op(call.clone()),
        "CallDirect: callee return slot 0 [0, 8) exceeds the callee region (0)",
    );
    assert_accepted(&Function {
        extended_frame_size: 56,
        ..func_with_single_op(call)
    });
}

// ---------------------------------------------------------------------------
// Closures
// ---------------------------------------------------------------------------

fn pack_closure(op: PackClosureOp) -> Function {
    Function {
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..func_with_single_op(MicroOp::PackClosure(Box::new(op)))
    }
}

/// A closure over `one_param_callee` capturing its single `u64` parameter
/// from frame slot 8.
fn capturing_closure() -> PackClosureOp {
    PackClosureOp {
        dst: FO(0),
        func_ref: ClosureFuncRef::Resolved(FunctionPtr::new(Box::new(one_param_callee()))),
        mask: 0b1,
        captured_data_descriptor_id: Some(TRIVIAL_DESCRIPTOR_ID),
        values_size: 8,
        captured: vec![slot(8, 8, 8)],
    }
}

#[test]
fn pack_closure_well_formed_accepted() {
    assert_accepted(&pack_closure(capturing_closure()));
}

#[test]
fn pack_closure_descriptor_and_captures_must_agree() {
    assert_error_contains(
        &pack_closure(PackClosureOp {
            captured: vec![],
            values_size: 0,
            mask: 0,
            ..capturing_closure()
        }),
        "provided but no captures",
    );
    assert_error_contains(
        &pack_closure(PackClosureOp {
            captured_data_descriptor_id: None,
            ..capturing_closure()
        }),
        "captured_data_descriptor_id is None",
    );
    assert_error_contains(
        &pack_closure(PackClosureOp {
            captured_data_descriptor_id: Some(POINTER_VEC_DESCRIPTOR_ID),
            ..capturing_closure()
        }),
        "is not a Trivial or CapturedData",
    );
}

#[test]
fn pack_closure_captured_data_pointers_must_lie_inside_the_values() {
    let mut descriptors = ObjectDescriptorTable::new();
    // Offsets 0 and 8 need 16 bytes of values; the closure only has 8.
    let captured_data =
        descriptors.push(ObjectDescriptor::new_captured_data(16, vec![0, 8]).unwrap());
    let op = PackClosureOp {
        captured_data_descriptor_id: Some(captured_data),
        ..capturing_closure()
    };
    let errors = check_well_formedness(
        &pack_closure(op),
        &TestProvider::new(descriptors),
        NO_CONSTANTS,
    );
    assert!(
        errors.iter().any(|e| e
            .message
            .contains("pointer offset 8 out of bounds of values_size 8")),
        "{errors:#?}"
    );
}

#[test]
fn pack_closure_mask_and_captured_list_must_match_the_callee() {
    assert_error_contains(
        &pack_closure(PackClosureOp {
            mask: 0b11,
            ..capturing_closure()
        }),
        "captured list length 1 does not match mask captured count 2",
    );
    assert_error_contains(
        &pack_closure(PackClosureOp {
            mask: 0b10,
            ..capturing_closure()
        }),
        "mask 0x2 sets bits beyond callee param count 1",
    );
    assert_error_contains(
        &pack_closure(PackClosureOp {
            captured: vec![slot(8, 4, 4)],
            values_size: 4,
            ..capturing_closure()
        }),
        "captured[0].size 4 != callee param_slots[0].size 8",
    );
    assert_error_contains(
        &pack_closure(PackClosureOp {
            captured: vec![slot(8, 8, 4)],
            ..capturing_closure()
        }),
        "captured[0].align 4 != callee param_slots[0].align 8",
    );
}

#[test]
fn pack_closure_values_size_and_captured_alignment_are_checked() {
    assert_error_contains(
        &pack_closure(PackClosureOp {
            values_size: 16,
            ..capturing_closure()
        }),
        "values_size 16 != captured layout size 8",
    );
    assert_error_contains(
        &pack_closure(PackClosureOp {
            captured: vec![slot(8, 8, 0)],
            ..capturing_closure()
        }),
        "PackClosure: captured[0]: align 0 must be a power of two",
    );
}

#[test]
fn call_closure_provided_args_are_checked() {
    let call = |provided_args| {
        func_with_single_op(MicroOp::CallClosure(Box::new(CallClosureOp {
            closure_src: FO(0),
            provided_args,
        })))
    };
    assert_accepted(&call(vec![slot(8, 8, 8)]));
    assert_error_contains(
        &call(vec![slot(8, 0, 8)]),
        "provided_args[0]: size must be > 0",
    );
    assert_error_contains(&call(vec![slot(8, 8, 3)]), "align 3 must be a power of two");
    assert_error_contains(&call(vec![slot(20, 8, 8)]), "overlaps metadata");
}

// ---------------------------------------------------------------------------
// Signature layout and entry root coverage
// ---------------------------------------------------------------------------

#[test]
fn signatures_require_type_sizes_alignments_and_canonical_offsets() {
    for bad_slot in [slot(0, 1, 1), slot(0, 8, 1), slot(8, 8, 8)] {
        let parameter = Function {
            param_slots: vec![bad_slot],
            param_region_size: 16,
            param_and_local_sizes_sum: 16,
            extended_frame_size: 40,
            ..one_param_callee()
        };
        let returned = Function {
            return_slots: vec![bad_slot],
            return_tys: one_u64(),
            param_and_local_sizes_sum: 16,
            extended_frame_size: 40,
            ..minimal_func()
        };
        let error = if bad_slot.offset.0 == 8 {
            "offset 8 != signature layout offset 0"
        } else {
            "!= type layout (8, 8)"
        };
        assert_error_contains(&parameter, error);
        assert_error_contains(&returned, error);
    }
}

#[test]
fn signature_padding_and_parameter_return_storage_reuse_are_valid() {
    static TYS: [InternedType; 3] = [U8_TY, U64_TY, U8_TY];
    let slots = vec![slot(0, 1, 1), slot(8, 8, 8), slot(16, 1, 1)];
    let func = Function {
        param_slots: slots.clone(),
        param_tys: TYS.to_vec(),
        return_slots: slots,
        return_tys: InternedTypeList::new(GlobalArenaPtr::from_static(&TYS[..])),
        param_region_size: 17,
        param_and_local_sizes_sum: 24,
        extended_frame_size: 48,
        ..minimal_func()
    };
    assert_accepted(&func);
    assert_error_contains(
        &Function {
            param_region_size: 24,
            ..func
        },
        "must equal parameter signature layout size (17)",
    );
    assert_error_contains(
        &Function {
            param_region_size: 8,
            ..minimal_func()
        },
        "must equal parameter signature layout size (0)",
    );
}

#[test]
fn signature_types_must_have_known_shapes() {
    static UNKNOWN: Type = Type::TypeParam { idx: 0 };
    static TYS: [InternedType; 1] = [GlobalArenaPtr::from_static(&UNKNOWN)];
    let func = Function {
        param_tys: TYS.to_vec(),
        return_tys: InternedTypeList::new(GlobalArenaPtr::from_static(&TYS[..])),
        return_slots: vec![slot(0, 8, 8)],
        ..one_param_callee()
    };
    assert_error_contains(&func, "param type 0 has no known size and alignment");
    assert_error_contains(&func, "return type 0 has no known size and alignment");
}

#[test]
fn base_roots_must_fit_one_parameter_not_padding_or_two_parameters() {
    for (params, tys, end) in [
        (vec![slot(0, 1, 1)], vec![U8_TY], 1),
        (vec![slot(0, 1, 1), slot(8, 8, 8)], vec![U8_TY, U64_TY], 16),
        (vec![slot(0, 4, 4), slot(4, 4, 4)], vec![U32_TY, U32_TY], 8),
    ] {
        for zero_frame in [false, true] {
            let func = Function {
                param_slots: params.clone(),
                param_tys: tys.clone(),
                param_region_size: end,
                param_and_local_sizes_sum: 16,
                extended_frame_size: 40,
                zero_frame,
                frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
                ..minimal_func()
            };
            // Each signature is valid; the sole error is root coverage.
            let errors = errors_of(&func);
            assert_eq!(errors.len(), 1, "{errors:#?}");
            assert!(errors[0].contains("not inside one parameter slot"));
        }
    }
    assert_accepted(&Function {
        param_slots: vec![slot(0, 16, 8)],
        param_tys: vec![ref_u64()],
        param_region_size: 16,
        param_and_local_sizes_sum: 16,
        extended_frame_size: 40,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..minimal_func()
    });
}

#[test]
fn fat_pointer_numeric_half_is_checked_for_reads_and_writes() {
    for op in [
        MicroOp::SlotBorrow {
            dst: FO(0),
            local: FO(16),
        },
        MicroOp::ReadRef {
            ref_ptr: FO(0),
            dst: FO(16),
            size: 8,
        },
    ] {
        let func = Function {
            zero_frame: true,
            frame_layout: FrameLayoutInfo::new(vec![FO(8)]),
            ..func_with_single_op(op)
        };
        assert_error_contains(&func, "access [8, 16) aliases frame_layout pointer slot 8");
        assert_accepted(&Function {
            frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
            ..func
        });
    }
    let func = Function {
        safe_point_layouts: SortedSafePointEntries::new(vec![SafePointEntry {
            code_offset: CO(0),
            layout: FrameLayoutInfo::new(vec![FO(8)]),
        }]),
        ..func_with_single_op(MicroOp::VecPushBack {
            vec_ref: FO(0),
            elem: FO(16),
            elem_size: 8,
            descriptor_id: TRIVIAL_DESCRIPTOR_ID,
        })
    };
    assert_error_contains(&func, "access [8, 16) aliases safe-point pointer slot 8");
}

#[test]
fn typed_values_and_constants_cannot_alias_gc_roots() {
    let cmp = Function {
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(0)]),
        ..func_with_single_op(MicroOp::ValueCmp(ValueCmpOp {
            dst: FO(16),
            lhs: FO(0),
            rhs: FO(8),
            ty: U64_TY,
            negate: false,
        }))
    };
    assert_error_contains(&cmp, "access [0, 8) aliases frame_layout pointer slot 0");
    for safe_point in [false, true] {
        let mut constant = func_with_single_op(MicroOp::StoreImmVec {
            dst: FO(0),
            idx: ConstantPoolIndex(0),
        });
        if safe_point {
            constant.safe_point_layouts = SortedSafePointEntries::new(vec![SafePointEntry {
                code_offset: CO(0),
                layout: FrameLayoutInfo::new(vec![FO(0)]),
            }]);
        } else {
            constant.zero_frame = true;
            constant.frame_layout = FrameLayoutInfo::new(vec![FO(0)]);
        }
        let errors = check_well_formedness(&constant, &trivial_descriptors(), &[U64_TY][..]);
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert!(errors[0].message.contains("access [0, 8) aliases"));
    }
}

#[test]
fn non_pointer_aggregate_layout_flag_applies_to_values_and_constants() {
    use mono_move_core::{FieldValueLayout, LayoutFlags};
    let ty = GlobalArenaPtr::from_static(Box::leak(Box::new(Type::Nominal {
        module_id: test_module_id(),
        name: GlobalArenaPtr::from_static("Pair"),
        ty_args: EMPTY_TYPE_LIST,
    })));
    let mut provider = trivial_descriptors();
    let field_id = provider.layout_id(U64_TY).unwrap();
    provider.layouts.push(ValueLayout::struct_layout(
        Some(ty),
        16,
        8,
        Some(16),
        LayoutFlags::NO_POINTERS_NO_PADDING,
        vec![
            FieldValueLayout {
                offset: 0,
                id: field_id,
            },
            FieldValueLayout {
                offset: 8,
                id: field_id,
            },
        ]
        .into_boxed_slice(),
    ));
    for op in [
        MicroOp::ValueCmp(ValueCmpOp {
            dst: FO(32),
            lhs: FO(0),
            rhs: FO(16),
            ty,
            negate: false,
        }),
        MicroOp::StoreImmVec {
            dst: FO(0),
            idx: ConstantPoolIndex(0),
        },
    ] {
        let func = Function {
            param_and_local_sizes_sum: 40,
            extended_frame_size: 64,
            zero_frame: true,
            frame_layout: FrameLayoutInfo::new(vec![FO(8)]),
            ..func_with_single_op(op)
        };
        let errors = check_well_formedness(&func, &provider, &[ty][..]);
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert!(errors[0]
            .message
            .contains("access [0, 16) aliases frame_layout pointer slot 8"));
    }
}

#[test]
fn typed_reference_constant_has_a_non_pointer_offset_half() {
    let func = Function {
        zero_frame: true,
        frame_layout: FrameLayoutInfo::new(vec![FO(8)]),
        ..func_with_single_op(MicroOp::StoreImmVec {
            dst: FO(0),
            idx: ConstantPoolIndex(0),
        })
    };
    let errors = check_well_formedness(&func, &trivial_descriptors(), &[ref_u64()][..]);
    assert_eq!(errors.len(), 1, "{errors:#?}");
    assert!(errors[0]
        .message
        .contains("access [8, 16) aliases frame_layout pointer slot 8"));
}

fn comparison_ops(ty: InternedType) -> Vec<MicroOp> {
    vec![
        MicroOp::ValueCmp(ValueCmpOp {
            dst: FO(32),
            lhs: FO(0),
            rhs: FO(16),
            ty,
            negate: false,
        }),
        MicroOp::ValueRefCmp(ValueRefCmpOp {
            dst: FO(32),
            lhs: FO(0),
            rhs: FO(16),
            ty,
            negate: false,
        }),
        MicroOp::JumpValueCmp(JumpValueCmpOp {
            lhs: FO(0),
            rhs: FO(16),
            ty,
            negate: false,
            target: CO(1),
            gas_taken: 0,
            gas_fallthrough: 0,
        }),
        MicroOp::JumpValueRefCmp(JumpValueRefCmpOp {
            lhs: FO(0),
            rhs: FO(16),
            ty,
            negate: false,
            target: CO(1),
            gas_taken: 0,
            gas_fallthrough: 0,
        }),
    ]
}

#[test]
fn all_comparisons_and_constants_require_published_layouts() {
    static VEC: Type = Type::Vector { elem: U64_TY };
    let ty = GlobalArenaPtr::from_static(&VEC);
    let mut provider = trivial_descriptors();
    assert_eq!(provider.size_and_align(ty), Some((8, 8)));
    let mut ops = comparison_ops(ty);
    ops.push(MicroOp::StoreImmVec {
        dst: FO(0),
        idx: ConstantPoolIndex(0),
    });
    for op in &ops {
        let func = Function {
            param_and_local_sizes_sum: 40,
            extended_frame_size: 64,
            ..func_with_single_op(op.clone())
        };
        let errors = check_well_formedness(&func, &provider, &[ty][..]);
        assert!(!errors.is_empty());
        assert!(
            errors
                .iter()
                .all(|e| e.message.contains("no published layout")),
            "{errors:#?}"
        );
    }
    let elem_id = provider.layout_id(U64_TY).unwrap();
    provider
        .layouts
        .push(ValueLayout::vector(ty, elem_id, TRIVIAL_DESCRIPTOR_ID));
    for op in ops {
        let func = Function {
            param_and_local_sizes_sum: 40,
            extended_frame_size: 64,
            ..func_with_single_op(op)
        };
        let errors = check_well_formedness(&func, &provider, &[ty][..]);
        assert!(errors.is_empty(), "{errors:#?}");
    }
}

#[test]
fn enum_field_tables_cannot_address_the_tag() {
    for offset in [0, 7, 8, 16] {
        let offsets = vec![None, Some(offset)].into_boxed_slice();
        for op in [
            MicroOp::EnumBorrowVariantFieldByTag {
                enum_ref: FO(0),
                dst: FO(16),
                offsets: offsets.clone(),
            },
            MicroOp::EnumReadVariantFieldByTag {
                enum_ref: FO(0),
                dst: FO(16),
                size: 8,
                offsets: offsets.clone(),
            },
            MicroOp::EnumWriteVariantFieldByTag {
                enum_ref: FO(0),
                src: FO(16),
                size: 8,
                offsets: offsets.clone(),
            },
        ] {
            let func = Function {
                param_and_local_sizes_sum: 32,
                extended_frame_size: 56,
                ..func_with_single_op(op)
            };
            if offset < 8 {
                assert_error_contains(&func, "is below ENUM_DATA_OFFSET (8)");
            } else {
                assert_accepted(&func);
            }
        }
    }
    assert_accepted(&func_with_single_op(MicroOp::HeapMoveTo8 {
        heap_ptr: FO(0),
        src: FO(8),
        offset: 0,
    }));
}

fn unresolved_capture(captured: Vec<SizedSlot>, values_size: u32) -> PackClosureOp {
    let function_ref = FunctionRef {
        module_id: test_module_id(),
        func_name: GlobalArenaPtr::from_static("callee"),
        ty_args: EMPTY_TYPE_LIST,
    };
    PackClosureOp {
        dst: FO(0),
        func_ref: ClosureFuncRef::Unresolved(GlobalArenaPtr::from_static(Box::leak(Box::new(
            function_ref,
        )))),
        mask: (1u64 << captured.len()) - 1,
        captured_data_descriptor_id: (!captured.is_empty()).then_some(TRIVIAL_DESCRIPTOR_ID),
        values_size,
        captured,
    }
}

#[test]
fn capture_size_accounts_for_alignment_not_source_offsets_or_final_padding() {
    assert_accepted(&pack_closure(unresolved_capture(vec![], 0)));
    assert_error_contains(
        &pack_closure(unresolved_capture(vec![], 1)),
        "!= captured layout size 0",
    );
    let captures = vec![slot(17, 1, 1), slot(8, 8, 8), slot(18, 1, 1)];
    assert_accepted(&pack_closure(unresolved_capture(captures.clone(), 17)));
    for wrong_size in [10, 24] {
        assert_error_contains(
            &pack_closure(unresolved_capture(captures.clone(), wrong_size)),
            "!= captured layout size 17",
        );
    }
}

#[test]
fn capture_size_cannot_wrap_during_alignment_or_addition() {
    for captures in [vec![slot(0, u32::MAX, 1), slot(0, 1, 1)], vec![
        slot(0, u32::MAX, 1),
        slot(0, 8, 8),
    ]] {
        let func = Function {
            param_and_local_sizes_sum: 1usize << 32,
            extended_frame_size: (1usize << 32) + 24,
            ..pack_closure(unresolved_capture(captures, 0))
        };
        let errors = errors_of(&func);
        assert_eq!(errors.len(), 1, "{errors:#?}");
        assert!(errors[0].contains("captured layout size exceeds u32"));
    }
}

#[test]
fn closure_sources_must_end_within_caller_data() {
    let call = |sources| Function {
        extended_frame_size: 64,
        ..func_with_single_op(MicroOp::CallClosure(Box::new(CallClosureOp {
            closure_src: FO(0),
            provided_args: sources,
        })))
    };
    assert_accepted(&call(vec![slot(16, 8, 8)])); // ends exactly at data boundary
    assert_error_contains(
        &call(vec![slot(56, 8, 8), slot(48, 8, 8)]),
        "beyond data region (24)",
    );
    assert_error_contains(&call(vec![slot(20, 8, 1)]), "beyond data region (24)");
    assert_accepted(&Function {
        extended_frame_size: 64,
        ..func_with_single_op(MicroOp::Move8 {
            src: FO(56),
            dst: FO(48),
        })
    });
}

#[test]
fn logical_destinations_reject_duplicates_and_allow_in_place_accesses() {
    assert_error_contains(
        &func_with_single_op(MicroOp::DeepCopyHeapPtrs {
            base: FO(8),
            offsets: vec![0, 0].into_boxed_slice(),
        }),
        "destinations [8, 16) and [8, 16) overlap",
    );
    assert_accepted(&func_with_single_op(MicroOp::DeepCopyHeapPtrs {
        base: FO(8),
        offsets: vec![8, 0].into_boxed_slice(),
    }));
    assert_error_contains(
        &func_with_single_op(MicroOp::VecUnpack(Box::new(VecUnpackOp {
            src: FO(16),
            dsts: vec![FO(0), FO(0)],
            elem_size: 8,
        }))),
        "destinations [0, 8) and [0, 8) overlap",
    );
    assert_accepted(&func_with_single_op(MicroOp::AddU64 {
        dst: FO(0),
        lhs: FO(0),
        rhs: FO(0),
    }));
    assert_accepted(&func_with_single_op(MicroOp::Move {
        dst: FO(4),
        src: FO(0),
        size: 8,
    }));
    assert_accepted(&func_with_single_op(MicroOp::HeapBorrow {
        dst: FO(0),
        obj_ref: FO(0),
        offset: 8,
    }));
}

#[test]
fn direct_call_checks_the_final_return_end() {
    let callee = Function {
        return_slots: vec![slot(0, 8, 8), slot(8, 8, 8)],
        return_tys: two_u64(),
        param_and_local_sizes_sum: 16,
        extended_frame_size: 40,
        ..minimal_func()
    };
    assert_accepted(&callee);
    let call = MicroOp::CallDirect {
        ptr: FunctionPtr::new(Box::new(callee)),
    };
    let caller = Function {
        extended_frame_size: 56,
        ..func_with_single_op(call)
    };
    assert_error_contains(
        &caller,
        "callee return slot 1 [8, 16) exceeds the callee region (8)",
    );
    assert_accepted(&Function {
        extended_frame_size: 64,
        ..caller
    });
}

// ---------------------------------------------------------------------------
// Specification / implementation consistency
// ---------------------------------------------------------------------------

/// A check id: an uppercase letter followed by digits (`F1`, `O6`).
fn is_check_id(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_uppercase())
        && !s[1..].is_empty()
        && chars.all(|c| c.is_ascii_digit())
}

/// Expands `F1, P2-P4` into `{F1, P2, P3, P4}`, or `None` if `list` is not a
/// comma-separated list of ids and id ranges.
fn expand_ids(list: &str) -> Option<Vec<String>> {
    let mut ids = Vec::new();
    for item in list.split(',').map(str::trim) {
        match item.split_once('-') {
            Some((lo, hi)) if is_check_id(lo) && is_check_id(hi) && lo[..1] == hi[..1] => {
                let (a, b): (u32, u32) = (lo[1..].parse().ok()?, hi[1..].parse().ok()?);
                ids.extend((a..=b).map(|n| format!("{}{n}", &lo[..1])));
            },
            None if is_check_id(item) => ids.push(item.to_string()),
            _ => return None,
        }
    }
    Some(ids)
}

/// Every check in the spec tables is cited at least once where it is
/// evaluated, and every cited id is specified. A spec row is a module doc
/// line `//! | F1 | ...`; a citation is a `//` or `///` comment opening with a
/// bracketed id list such as `[F1]` or `[P1-P4, R1-R4]`.
#[test]
fn every_specified_check_is_implemented_and_vice_versa() {
    use std::collections::BTreeSet;
    let source = include_str!("../src/well_formedness.rs");
    let mut specified = BTreeSet::new();
    let mut cited = BTreeSet::new();
    for line in source.lines().map(str::trim_start) {
        if let Some(row) = line.strip_prefix("//! | ") {
            let id = row.split(' ').next().unwrap_or("");
            if is_check_id(id) {
                specified.insert(id.to_string());
            }
        } else if let Some(comment) = line.strip_prefix("//").filter(|c| !c.starts_with('!')) {
            let body = comment.trim_start_matches('/').trim_start();
            if let Some(list) = body.strip_prefix('[').and_then(|b| b.split(']').next()) {
                if let Some(ids) = expand_ids(list) {
                    cited.extend(ids);
                }
            }
        }
    }
    assert!(!specified.is_empty() && !cited.is_empty());
    let unimplemented: Vec<_> = specified.difference(&cited).collect();
    let unspecified: Vec<_> = cited.difference(&specified).collect();
    assert!(
        unimplemented.is_empty(),
        "specified but never cited: {unimplemented:?}"
    );
    assert!(
        unspecified.is_empty(),
        "cited but not in the spec: {unspecified:?}"
    );
}
