// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Microbenchmarks for `Drop` on Move `Value`s. The interesting variants are
// `Container::{Vec, Struct, Locals}` which the iterative-Drop work-stack walks.
//
// All builders use the public `Value` / `Struct` API, which routes through
// `NestedValues::new(...)` internally, so each value is uniquely owned and
// Drop walks the unique-Rc path (the one we want to measure).

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use move_vm_types::values::{Struct, Value};

/// `vector<Color>` of length `len` where `Color = { r: u8, g: u8, b: u8 }`.
/// Mirrors the shape of `vector_picture`'s `Palette.vec`.
fn make_color_vec(len: usize) -> Value {
    Value::vector_unchecked((0..len).map(|i| {
        let b = (i & 0xFF) as u8;
        Value::struct_(Struct::pack([Value::u8(b), Value::u8(b), Value::u8(b)]))
    }))
    .unwrap()
}

/// `Palette { vec: vector<Color> }` — one wrapper struct around the color vec.
fn make_palette(len: usize) -> Value {
    Value::struct_(Struct::pack([make_color_vec(len)]))
}

/// `vector<vector<u64>>` — outer fanout × inner fanout. Both levels go through
/// `Container::Vec(NestedValues)` (the inner `vector<u64>` is `VecU64`, which
/// is the specialized primitive variant — so depth=1 from the Drop walker's POV).
fn make_vec_of_vec_u64(outer: usize, inner: usize) -> Value {
    Value::vector_unchecked((0..outer).map(|_| Value::vector_u64((0..inner).map(|j| j as u64))))
        .unwrap()
}

/// `vector<vector<S>>` where `S = { u8 }`. Both nesting levels are
/// `NestedValues`, so every element pushes onto the iterative-Drop work stack.
/// This is the shape that maximally exercises the BFS stack growth.
fn make_vec_of_vec_struct(outer: usize, inner: usize) -> Value {
    Value::vector_unchecked((0..outer).map(|_| {
        Value::vector_unchecked(
            (0..inner).map(|i| Value::struct_(Struct::pack([Value::u8((i & 0xFF) as u8)]))),
        )
        .unwrap()
    }))
    .unwrap()
}

/// Right-deep nesting: `S { S { S { ... { u8 } } } }` of depth `depth`.
/// Each level is one `Container::Struct(NestedValues)` holding one child.
/// Useful as a regression check that DFS doesn't lose its depth handling.
fn make_deep_nested(depth: usize) -> Value {
    let mut v = Value::u8(0);
    for _ in 0..depth {
        v = Value::struct_(Struct::pack([v]));
    }
    v
}

fn bench_drop(c: &mut Criterion) {
    let mut group = c.benchmark_group("drop");
    group
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(3));

    // VectorPicture30K shape: 30,720 Colors, each a 3-u8 struct.
    group.bench_function("palette/30k", |b| {
        b.iter_batched(|| make_palette(30 * 1024), drop, BatchSize::PerIteration);
    });

    group.bench_function("color_vec/30k", |b| {
        b.iter_batched(|| make_color_vec(30 * 1024), drop, BatchSize::PerIteration);
    });

    group.bench_function("color_vec/1k", |b| {
        b.iter_batched(|| make_color_vec(1024), drop, BatchSize::SmallInput);
    });

    // Outer-wide × inner-narrow with primitive inner. Outer push goes through
    // NestedValues (BFS-stack territory); inner is `VecU64` (compiler drop).
    group.bench_function("vec_of_vec_u64/1000x100", |b| {
        b.iter_batched(
            || make_vec_of_vec_u64(1000, 100),
            drop,
            BatchSize::PerIteration,
        );
    });

    // Both levels are NestedValues. Maximally stresses the work stack.
    group.bench_function("vec_of_vec_struct/1000x30", |b| {
        b.iter_batched(
            || make_vec_of_vec_struct(1000, 30),
            drop,
            BatchSize::PerIteration,
        );
    });

    // Deep right-nested: depth bound matters, fanout = 1.
    group.bench_function("deep_nested/depth_1000", |b| {
        b.iter_batched(|| make_deep_nested(1000), drop, BatchSize::SmallInput);
    });

    group.finish();
}

criterion_group!(benches, bench_drop);
criterion_main!(benches);
