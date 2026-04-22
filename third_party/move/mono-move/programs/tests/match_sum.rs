// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_programs::match_sum::{native_match_sum, MATCH_SUM_CASES};

#[test]
fn native() {
    for &(n, expected) in MATCH_SUM_CASES {
        assert_eq!(native_match_sum(n), expected, "native_match_sum({n})");
    }
}

#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_core::NoopTransactionContext;
    use mono_move_gas::SimpleGasMeter;
    use mono_move_programs::match_sum::{micro_op_match_sum, MATCH_SUM_CASES};
    use mono_move_runtime::InterpreterContext;

    fn run(n: u64) -> u64 {
        let (functions, descriptors, _arena) = micro_op_match_sum();
        let txn_ctx = NoopTransactionContext;
        let gas_meter = SimpleGasMeter::new(u64::MAX);
        let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
            functions[0].as_ref_unchecked()
        });
        ctx.set_root_arg(0, &n.to_le_bytes());
        ctx.run().unwrap();
        ctx.root_result()
    }

    #[test]
    fn correctness() {
        for &(n, expected) in MATCH_SUM_CASES {
            assert_eq!(run(n), expected, "micro_op match_sum({n})");
        }
    }
}

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use super::MATCH_SUM_CASES;
    use mono_move_programs::{match_sum::move_bytecode_match_sum, testing};

    fn run(n: u64) -> u64 {
        let module = move_bytecode_match_sum();
        let result = testing::run_move_function(&module, "match_sum", vec![testing::arg_u64(n)]);
        testing::return_u64(&result)
    }

    #[test]
    fn correctness() {
        for &(n, expected) in MATCH_SUM_CASES {
            assert_eq!(run(n), expected, "move_bytecode match_sum({n})");
        }
    }
}
