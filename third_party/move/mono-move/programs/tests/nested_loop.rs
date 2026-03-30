// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_programs::nested_loop::{native_nested_loop, NESTED_LOOP_CASES};

#[test]
fn native() {
    for &(n, expected) in NESTED_LOOP_CASES {
        assert_eq!(native_nested_loop(n), expected, "native_nested_loop({n})");
    }
}

#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_programs::nested_loop::{micro_op_nested_loop, NESTED_LOOP_CASES};
    use mono_move_runtime::InterpreterContext;

    fn run(n: u64) -> u64 {
        let (functions, descriptors, _arena) = micro_op_nested_loop();
        let mut ctx =
            InterpreterContext::new(&descriptors, unsafe { functions[0].as_ref_unchecked() });
        ctx.set_root_arg(0, &n.to_le_bytes());
        ctx.run().unwrap();
        ctx.root_result()
    }

    #[test]
    fn correctness() {
        for &(n, expected) in NESTED_LOOP_CASES {
            assert_eq!(run(n), expected, "micro_op nested_loop({n})");
        }
    }
}

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use super::NESTED_LOOP_CASES;
    use mono_move_programs::{nested_loop::move_bytecode_nested_loop, testing};

    fn run(n: u64) -> u64 {
        let module = move_bytecode_nested_loop();
        let result = testing::run_move_function(&module, "nested_loop", vec![testing::arg_u64(n)]);
        testing::return_u64(&result)
    }

    #[test]
    fn correctness() {
        for &(n, expected) in NESTED_LOOP_CASES {
            assert_eq!(run(n), expected, "move_bytecode nested_loop({n})");
        }
    }
}
