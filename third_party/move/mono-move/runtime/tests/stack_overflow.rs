// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Call-stack overflows must be detected without forming an out-of-bounds
//! pointer.

mod common;

use mono_move_alloc::GlobalArenaPtr;
use mono_move_core::{
    native::NativeExtensions, types::EMPTY_TYPE_LIST, Code, FrameLayoutInfo, Function,
    FunctionDefinitionIndex, FunctionPtr, MicroOp, SortedSafePointEntries, VMInternalError,
    FRAME_METADATA_SIZE,
};
use mono_move_runtime::RuntimeError;

fn function(code: Vec<MicroOp>, locals_size: usize) -> Function {
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
        param_and_local_sizes_sum: locals_size,
        extended_frame_size: locals_size + FRAME_METADATA_SIZE,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(),
        safe_point_layouts: SortedSafePointEntries::empty(),
    }
}

/// Uses a `usize::MAX` frame to exercise both stack bounds and integer
/// overflow in frame-end calculations.
fn unfittable(code: Vec<MicroOp>) -> Function {
    Function {
        extended_frame_size: usize::MAX,
        ..function(code, 8)
    }
}

/// Returns any error from preparing or running `entry`.
fn call_error(entry: &Function) -> Option<VMInternalError> {
    common::with_test_interpreter(entry, u64::MAX, NativeExtensions::new(), |ctx, entry| {
        ctx.build_call(entry).and_then(|call| Ok(call.run()?)).err()
    })
}

fn assert_stack_overflow(err: Option<VMInternalError>) {
    let err = err.expect("the call should fail");
    assert!(
        matches!(
            err.downcast_ref::<RuntimeError>(),
            Some(RuntimeError::StackOverflow)
        ),
        "{err:?}"
    );
}

#[test]
fn call_with_a_frame_larger_than_the_stack() {
    let callee = FunctionPtr::new(Box::new(unfittable(vec![MicroOp::Return])));
    let entry = function(
        vec![MicroOp::CallDirect { ptr: callee }, MicroOp::Return],
        8,
    );
    let err = call_error(&entry);
    // SAFETY: The interpreter has been dropped, so no callee references remain.
    // This is the callee's only deallocation.
    unsafe { callee.free_unchecked() };
    assert_stack_overflow(err);
}

#[test]
fn entry_with_a_frame_larger_than_the_stack() {
    assert_stack_overflow(call_error(&unfittable(vec![MicroOp::Return])));
}
