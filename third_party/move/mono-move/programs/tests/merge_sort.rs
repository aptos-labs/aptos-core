// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_programs::merge_sort::{native_merge_sort, shuffled_range};

#[test]
fn native() {
    for &n in &[0, 1, 2, 10, 100] {
        let mut v = shuffled_range(n, 42);
        native_merge_sort(&mut v);
        let expected: Vec<u64> = (0..n).collect();
        assert_eq!(v, expected, "native_merge_sort(n={n})");
    }
}

#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_core::NoopTransactionContext;
    use mono_move_gas::SimpleGasMeter;
    use mono_move_programs::merge_sort::{micro_op_merge_sort, shuffled_range};
    use mono_move_runtime::{read_u64, InterpreterContext, VEC_DATA_OFFSET};

    fn run(n: u64) -> Vec<u64> {
        let values = shuffled_range(n, 42);
        let (functions, descriptors, _arena) = micro_op_merge_sort();
        // SAFETY: Exclusive access during test setup; arena is alive.
        unsafe { mono_move_core::Function::resolve_calls(&functions) };
        let txn_ctx = NoopTransactionContext;
        let gas_meter = SimpleGasMeter::new(u64::MAX);
        let mut ctx = InterpreterContext::new(&txn_ctx, &descriptors, gas_meter, unsafe {
            functions[0].unwrap().as_ref_unchecked()
        });
        let vec_ptr = ctx
            .alloc_u64_vec(mono_move_core::DescriptorId(0), &values)
            .unwrap();
        ctx.set_root_arg(0, &vec_ptr.to_le_bytes());
        ctx.run().unwrap();

        let heap_ptr = ctx.root_heap_ptr(0);
        (0..n)
            .map(|i| unsafe { read_u64(heap_ptr, VEC_DATA_OFFSET + i as usize * 8) })
            .collect()
    }

    #[test]
    fn correctness() {
        for &n in &[0, 1, 2, 10, 100] {
            let result = run(n);
            let expected: Vec<u64> = (0..n).collect();
            assert_eq!(result, expected, "micro_op merge_sort(n={n})");
        }
    }
}

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use mono_move_programs::{
        merge_sort::{move_bytecode_merge_sort, shuffled_range},
        testing,
    };

    fn run(n: u64) -> Vec<u64> {
        let values = shuffled_range(n, 42);
        let module = move_bytecode_merge_sort();
        let result =
            testing::run_move_function(&module, "merge_sort", vec![testing::arg_vec_u64(&values)]);
        testing::return_vec_u64(&result)
    }

    #[test]
    fn correctness() {
        for &n in &[0, 1, 2, 10, 100] {
            let result = run(n);
            let expected: Vec<u64> = (0..n).collect();
            assert_eq!(result, expected, "move_bytecode merge_sort(n={n})");
        }
    }
}
