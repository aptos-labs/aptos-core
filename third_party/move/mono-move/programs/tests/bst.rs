// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_programs::bst::BstMap;

// ---------------------------------------------------------------------------
// Native Rust
// ---------------------------------------------------------------------------

#[test]
fn native_insert_and_get() {
    let mut map = BstMap::new();
    assert_eq!(map.get(1), None);

    map.insert(5, 50);
    map.insert(3, 30);
    map.insert(7, 70);
    map.insert(1, 10);
    map.insert(4, 40);

    assert_eq!(map.get(5), Some(50));
    assert_eq!(map.get(3), Some(30));
    assert_eq!(map.get(7), Some(70));
    assert_eq!(map.get(1), Some(10));
    assert_eq!(map.get(4), Some(40));
    assert_eq!(map.get(99), None);
}

#[test]
fn native_insert_overwrite() {
    let mut map = BstMap::new();
    map.insert(5, 50);
    assert_eq!(map.get(5), Some(50));
    map.insert(5, 99);
    assert_eq!(map.get(5), Some(99));
}

#[test]
fn native_remove_leaf() {
    let mut map = BstMap::new();
    map.insert(5, 50);
    map.insert(3, 30);
    map.insert(7, 70);
    map.remove(3);
    assert_eq!(map.get(3), None);
    assert_eq!(map.get(5), Some(50));
    assert_eq!(map.get(7), Some(70));
}

#[test]
fn native_remove_one_child() {
    let mut map = BstMap::new();
    map.insert(5, 50);
    map.insert(3, 30);
    map.insert(7, 70);
    map.insert(6, 60);
    map.remove(7);
    assert_eq!(map.get(7), None);
    assert_eq!(map.get(6), Some(60));
}

#[test]
fn native_remove_two_children() {
    let mut map = BstMap::new();
    map.insert(5, 50);
    map.insert(3, 30);
    map.insert(7, 70);
    map.insert(6, 60);
    map.insert(8, 80);
    map.remove(7);
    assert_eq!(map.get(7), None);
    assert_eq!(map.get(6), Some(60));
    assert_eq!(map.get(8), Some(80));
}

#[test]
fn native_remove_root() {
    let mut map = BstMap::new();
    map.insert(5, 50);
    map.insert(3, 30);
    map.insert(7, 70);
    map.remove(5);
    assert_eq!(map.get(5), None);
    assert_eq!(map.get(3), Some(30));
    assert_eq!(map.get(7), Some(70));
}

#[test]
fn native_remove_nonexistent() {
    let mut map = BstMap::new();
    map.insert(5, 50);
    map.remove(99);
    assert_eq!(map.get(5), Some(50));
}

#[test]
fn native_insert_remove_sequence() {
    let mut map = BstMap::new();
    for i in 0..100 {
        map.insert(i, i * 10);
    }
    for i in 0..100 {
        assert_eq!(map.get(i), Some(i * 10));
    }
    for i in (0..100).step_by(2) {
        map.remove(i);
    }
    for i in 0..100 {
        if i % 2 == 0 {
            assert_eq!(map.get(i), None);
        } else {
            assert_eq!(map.get(i), Some(i * 10));
        }
    }
}

// ---------------------------------------------------------------------------
// Micro-op — cross-checked against native via random ops
// ---------------------------------------------------------------------------

#[cfg(feature = "micro-op")]
mod micro_op {
    use mono_move_programs::bst::{
        generate_ops, micro_op_bst, native_run_ops_with_results, FN_GET, FN_INSERT, FN_NEW,
        FN_REMOVE,
    };
    use mono_move_runtime::InterpreterContext;

    fn bst_new(ctx: &mut InterpreterContext) -> u64 {
        ctx.invoke(FN_NEW);
        ctx.run().unwrap();
        ctx.root_heap_ptr(0) as u64
    }

    fn bst_insert(ctx: &mut InterpreterContext, bst: u64, key: u64, value: u64) {
        ctx.invoke(FN_INSERT);
        ctx.set_root_arg(0, &bst.to_le_bytes());
        ctx.set_root_arg(8, &key.to_le_bytes());
        ctx.set_root_arg(16, &value.to_le_bytes());
        ctx.run().unwrap();
    }

    fn bst_get(ctx: &mut InterpreterContext, bst: u64, key: u64) -> (u64, u64) {
        ctx.invoke(FN_GET);
        ctx.set_root_arg(0, &bst.to_le_bytes());
        ctx.set_root_arg(8, &key.to_le_bytes());
        ctx.run().unwrap();
        let found = ctx.root_result();
        let value = ctx.root_result_at(8);
        (found, value)
    }

    fn bst_remove(ctx: &mut InterpreterContext, bst: u64, key: u64) {
        ctx.invoke(FN_REMOVE);
        ctx.set_root_arg(0, &bst.to_le_bytes());
        ctx.set_root_arg(8, &key.to_le_bytes());
        ctx.run().unwrap();
    }

    /// Run the same ops on micro-op BST and return results in the same format
    /// as `native_run_ops_with_results`.
    fn micro_op_run_ops_with_results(ops: &[u64]) -> Vec<(u64, u64)> {
        let (functions, descriptors) = micro_op_bst();
        let mut ctx = InterpreterContext::new(&functions, &descriptors, FN_NEW);
        let bst = bst_new(&mut ctx);
        let mut results = Vec::new();
        let mut i = 0;
        while i < ops.len() {
            let op = ops[i];
            let key = ops[i + 1];
            let value = ops[i + 2];
            i += 3;
            match op {
                0 => bst_insert(&mut ctx, bst, key, value),
                1 => results.push(bst_get(&mut ctx, bst, key)),
                2 => bst_remove(&mut ctx, bst, key),
                _ => {},
            }
        }
        results
    }

    /// Cross-check: random ops produce identical get results on native and micro-op.
    #[test]
    fn cross_check_vs_native() {
        let ops = generate_ops(500, 100, 42);
        let native_results = native_run_ops_with_results(&ops);
        let micro_op_results = micro_op_run_ops_with_results(&ops);
        assert_eq!(native_results, micro_op_results);
    }
}

// ---------------------------------------------------------------------------
// Move bytecode — cross-checked via run_ops (no per-op results, but verifies
// the Move implementation doesn't crash on the same workload)
// ---------------------------------------------------------------------------

#[cfg(feature = "move-bytecode")]
mod move_bytecode {
    use mono_move_programs::{
        bst::{generate_ops, move_bytecode_bst, move_stdlib_vector},
        testing,
    };

    #[test]
    fn run_ops() {
        let ops = generate_ops(500, 100, 42);
        let bst_module = move_bytecode_bst();
        let vector_module = move_stdlib_vector();
        testing::with_loaded_move_function_with_deps(
            &bst_module,
            &[&vector_module],
            "run_ops",
            |runner| {
                runner.run(vec![testing::arg_vec_u64(&ops)]);
            },
        );
    }
}
