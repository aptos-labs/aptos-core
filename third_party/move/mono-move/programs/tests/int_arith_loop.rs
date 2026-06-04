// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_programs::int_arith_loop::{native_i64_loop, native_u64_loop, TEST_ITERS};

#[test]
fn native_matches_across_flavors() {
    // The MUL/ADD/MOD constants are non-negative and `acc` starts at 1,
    // so u64 and i64 should evolve through the same sequence of values
    // and end at the same integer.
    let u = native_u64_loop(TEST_ITERS);
    let i = native_i64_loop(TEST_ITERS);
    assert!(i >= 0, "i64 loop went negative: {i}");
    assert_eq!(u as i64, i, "u64 / i64 native loops diverged");
}

#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_programs::int_arith_loop::{
        micro_op_i64_loop, micro_op_u64_loop, native_i64_loop, native_u64_loop, TEST_ITERS,
    };
    use mono_move_runtime::{testing::test_txn_ctx_max_budget, InterpreterContext};

    fn run_u64() -> u64 {
        let (functions, descriptors) = micro_op_u64_loop(false);
        let mut exec_ctx = test_txn_ctx_max_budget(descriptors);
        let mut ctx =
            InterpreterContext::new(&mut exec_ctx, unsafe { functions[0].as_ref_unchecked() });
        ctx.set_root_arg(0, &TEST_ITERS.to_le_bytes());
        ctx.run().unwrap();
        ctx.root_result()
    }

    fn run_i64() -> i64 {
        let (functions, descriptors) = micro_op_i64_loop(false);
        let mut exec_ctx = test_txn_ctx_max_budget(descriptors);
        let mut ctx =
            InterpreterContext::new(&mut exec_ctx, unsafe { functions[0].as_ref_unchecked() });
        ctx.set_root_arg(0, &TEST_ITERS.to_le_bytes());
        ctx.run().unwrap();
        // root_result reads the slot as u64; reinterpret as i64.
        ctx.root_result() as i64
    }

    #[test]
    fn micro_op_u64_matches_native() {
        assert_eq!(run_u64(), native_u64_loop(TEST_ITERS));
    }

    #[test]
    fn micro_op_i64_matches_native() {
        assert_eq!(run_i64(), native_i64_loop(TEST_ITERS));
    }
}

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use mono_move_programs::{
        int_arith_loop::{
            move_bytecode_int_arith_loop, native_i64_loop, native_u64_loop, TEST_ITERS,
        },
        testing,
    };

    #[test]
    fn move_vm_u64_matches_native() {
        let module = move_bytecode_int_arith_loop();
        testing::with_loaded_move_function(&module, "u64_loop", |env| {
            let result = env.run(vec![testing::arg_u64(TEST_ITERS)]);
            assert_eq!(testing::return_u64(&result), native_u64_loop(TEST_ITERS));
        });
    }

    #[test]
    fn move_vm_i64_matches_native() {
        let module = move_bytecode_int_arith_loop();
        testing::with_loaded_move_function(&module, "i64_loop", |env| {
            let result = env.run(vec![testing::arg_u64(TEST_ITERS)]);
            assert_eq!(testing::return_i64(&result), native_i64_loop(TEST_ITERS));
        });
    }
}
