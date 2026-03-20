// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_programs::fib::{native_fib, FIB_CASES};

#[test]
fn native() {
    for &(n, expected) in FIB_CASES {
        assert_eq!(native_fib(n), expected, "native_fib({n})");
    }
}

#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_programs::fib::{micro_op_fib, FIB_CASES};
    use mono_move_runtime::InterpreterContext;

    fn run(n: u64) -> u64 {
        let (functions, descriptors) = micro_op_fib();
        let mut ctx = InterpreterContext::new(&functions, &descriptors, 0);
        ctx.set_root_arg(0, &n.to_le_bytes());
        ctx.run().unwrap();
        ctx.root_result()
    }

    #[test]
    fn correctness() {
        for &(n, expected) in FIB_CASES {
            assert_eq!(run(n), expected, "micro_op fib({n})");
        }
    }
}

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use super::FIB_CASES;
    use mono_move_programs::{fib::move_bytecode_fib, testing};

    fn run(n: u64) -> u64 {
        let module = move_bytecode_fib();
        let result = testing::run_move_function(&module, "fib", vec![testing::arg_u64(n)]);
        testing::return_u64(&result)
    }

    #[test]
    fn correctness() {
        for &(n, expected) in FIB_CASES {
            assert_eq!(run(n), expected, "move_bytecode fib({n})");
        }
    }
}
