// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-core/blob/main/LICENSE

//! Test for the cross-module call slow path (CallIndirect dispatched at
//! runtime via TransactionContext).
//!
//! TODO: Replace with a transactional test in mono-move/testsuite once the
//! loader is implemented and cross-module calls work end-to-end.

use mono_move_alloc::{ExecutableArena, ExecutableArenaPtr, GlobalArenaPtr};
use mono_move_core::{
    ExecutableId, FrameLayoutInfo, FrameOffset as FO, Function, FunctionResolver, MicroOp,
    SortedSafePointEntries, TransactionContext, FRAME_METADATA_SIZE,
};
use mono_move_gas::SimpleGasMeter;
use mono_move_runtime::InterpreterContext;
use move_core_types::account_address::AccountAddress;
use std::sync::LazyLock;

/// A test TransactionContext that resolves a single cross-module function.
struct TestTransactionContext {
    target_executable_id: GlobalArenaPtr<ExecutableId>,
    target_func_name: GlobalArenaPtr<str>,
    target_func_ptr: ExecutableArenaPtr<Function>,
}

impl FunctionResolver for TestTransactionContext {
    fn resolve_function(
        &self,
        executable_id: GlobalArenaPtr<ExecutableId>,
        name: GlobalArenaPtr<str>,
    ) -> Option<ExecutableArenaPtr<Function>> {
        if executable_id == self.target_executable_id && name == self.target_func_name {
            Some(self.target_func_ptr)
        } else {
            None
        }
    }
}

impl TransactionContext for TestTransactionContext {}

/// Verifies that the interpreter's runtime slow path correctly dispatches
/// cross-module calls through the TransactionContext.
///
/// ```move
/// module 0x1::foo {
///     public fun add_one(x: u64): u64 { x + 1 }
/// }
///
/// module 0x1::bar {
///     public fun main(): u64 { 0x1::foo::add_one(41) }
/// }
/// ```
#[test]
fn call_indirect_runtime_dispatch() {
    let arena = ExecutableArena::new();

    // module 0x1::foo {
    //     public fun add_one(x: u64): u64 { x + 1 }
    // }
    // Frame: [0..8) arg/return, [8..32) metadata
    let callee_name = GlobalArenaPtr::from_static("add_one");
    let callee_code = arena.alloc_slice_fill_iter(vec![
        MicroOp::AddU64Imm {
            dst: FO(0),
            src: FO(0),
            imm: 1,
        },
        MicroOp::Return,
    ]);
    let callee = arena.alloc(Function {
        name: callee_name,
        code: callee_code,
        args_size: 8,
        args_and_locals_size: 8,
        extended_frame_size: 8 + FRAME_METADATA_SIZE,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(&arena),
        safe_point_layouts: SortedSafePointEntries::empty(&arena),
    });

    static EXECUTABLE_ID: LazyLock<ExecutableId> = LazyLock::new(|| unsafe {
        ExecutableId::new(AccountAddress::ONE, GlobalArenaPtr::from_static("foo"))
    });
    let executable_id = GlobalArenaPtr::from_static(&*EXECUTABLE_ID);

    // module 0x1::bar {
    //     public fun main(): u64 { 0x1::foo::add_one(41) }
    // }
    // Frame: [0..8) result, [8..32) metadata, [32..40) callee arg slot
    let caller_name = GlobalArenaPtr::from_static("main");
    let caller_code = arena.alloc_slice_fill_iter(vec![
        // Store 41 into callee's arg slot (offset 32 = past metadata).
        MicroOp::StoreImm8 {
            dst: FO(8 + FRAME_METADATA_SIZE as u32),
            imm: 41,
        },
        // Cross-module call to foo::add_one.
        MicroOp::CallIndirect {
            executable_id,
            func_name: callee_name,
        },
        // Copy return value from callee's frame to our result slot.
        MicroOp::Move8 {
            dst: FO(0),
            src: FO(8 + FRAME_METADATA_SIZE as u32),
        },
        MicroOp::Return,
    ]);
    let caller = arena.alloc(Function {
        name: caller_name,
        code: caller_code,
        args_size: 0,
        args_and_locals_size: 8,
        extended_frame_size: 16 + FRAME_METADATA_SIZE,
        zero_frame: false,
        frame_layout: FrameLayoutInfo::empty(&arena),
        safe_point_layouts: SortedSafePointEntries::empty(&arena),
    });

    // -- Set up interpreter and run --
    let txn_ctx = TestTransactionContext {
        target_executable_id: executable_id,
        target_func_name: callee_name,
        target_func_ptr: callee,
    };
    let gas_meter = SimpleGasMeter::new(u64::MAX);
    let caller_ref = unsafe { caller.as_ref_unchecked() };
    let mut ctx = InterpreterContext::new(&txn_ctx, &[], gas_meter, caller_ref);
    ctx.run().expect("execution should succeed");

    assert_eq!(ctx.root_result(), 42, "expected foo::add_one(41) = 42");
}
