// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::mono::{
    code::MonoCode,
    context::{FunctionContext, FunctionId, MemoryBounds, MockExecutionContext},
    executor::Executor,
    primitives,
};
use mockall::predicate;

fn context_mock() -> MockExecutionContext {
    let mut mock = MockExecutionContext::new();
    mock.expect_heap_bounds().return_const(MemoryBounds {
        initial_capacity: 20,
        max_size: 100000,
    });
    mock.expect_stack_bounds().return_const(MemoryBounds {
        initial_capacity: 20,
        max_size: 100000,
    });
    mock
}

#[test]
pub fn identity() {
    const FUN_ID: FunctionId = FunctionId { function_hash: 1 };
    const FUN_CTX: FunctionContext = FunctionContext {
        id: FUN_ID,
        code: &[
            MonoCode::CopyLocal { local: 0, size: 8 },
            MonoCode::Return { size: 8 },
        ],
        params_size: 8,
        locals_size: 8,
        local_table: &[0],
    };
    let mut mock = context_mock();
    mock.expect_fetch_function()
        .with(predicate::eq(&FUN_ID))
        .returning(|_| Ok(FUN_CTX));

    let mut executor = Executor::new(&mock);
    let bytes = 72777u64.to_le_bytes().to_vec();
    let result = executor
        .execute(&mock, &FUN_ID, &bytes)
        .expect("no failure");
    assert_eq!(result, bytes)
}

#[test]
pub fn fibonacci() {
    const FUN_ID: FunctionId = FunctionId { function_hash: 1 };
    // Fib is defined as follows:
    //   fib(0) == 0, fib(1) == 1, fib(n) = fib(n-1) + fib(n-2)
    const FUN_CTX: FunctionContext = FunctionContext {
        id: FUN_ID,
        code: &[
            // if x == 0 return 0
            MonoCode::CopyLocal { local: 0, size: 8 },
            MonoCode::LoadConst {
                value: &0u64.to_le_bytes(),
            },
            MonoCode::CallPrimitive {
                size: 8,
                operation: primitives::equals,
            },
            // offset 3
            MonoCode::BranchFalse { offset: 3 + 3 },
            MonoCode::LoadConst {
                value: &0u64.to_le_bytes(),
            },
            MonoCode::Return { size: 8 },
            // if x == 1 return 1
            MonoCode::CopyLocal { local: 0, size: 8 },
            MonoCode::LoadConst {
                value: &1u64.to_le_bytes(),
            },
            MonoCode::CallPrimitive {
                size: 8,
                operation: primitives::equals,
            },
            // offset 9
            MonoCode::BranchFalse { offset: 9 + 3 },
            MonoCode::LoadConst {
                value: &1u64.to_le_bytes(),
            },
            MonoCode::Return { size: 8 },
            // else fib(n - 1) + fib(n - 2)
            // .. fib(n-1)
            MonoCode::CopyLocal { local: 0, size: 8 },
            MonoCode::LoadConst {
                value: &1u64.to_le_bytes(),
            },
            MonoCode::CallPrimitive {
                size: 8,
                operation: primitives::sub_u64,
            },
            MonoCode::CallFunction {
                function_id: FUN_ID,
            },
            // .. fib(n-2)
            MonoCode::CopyLocal { local: 0, size: 8 },
            MonoCode::LoadConst {
                value: &2u64.to_le_bytes(),
            },
            MonoCode::CallPrimitive {
                size: 8,
                operation: primitives::sub_u64,
            },
            MonoCode::CallFunction {
                function_id: FUN_ID,
            },
            // .. fib(n-1) + fib(n-2)
            MonoCode::CallPrimitive {
                size: 8,
                operation: primitives::add_u64,
            },
            MonoCode::Return { size: 8 },
        ],
        params_size: 8,
        locals_size: 8,
        local_table: &[0],
    };
    let mut mock = context_mock();
    mock.expect_fetch_function()
        .with(predicate::eq(&FUN_ID))
        .returning(|_| Ok(FUN_CTX));

    let mut executor = Executor::new(&mock);
    let input = 10u64.to_le_bytes().to_vec();
    let result = executor
        .execute(&mock, &FUN_ID, &input)
        .expect("no failure");
    assert_eq!(result, 55u64.to_le_bytes())
}
