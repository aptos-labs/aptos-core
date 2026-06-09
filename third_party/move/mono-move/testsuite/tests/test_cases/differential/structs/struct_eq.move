// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    fun eq(ax: u64, ay: u64, bx: u64, by: u64): bool {
        Point { x: ax, y: ay } == Point { x: bx, y: by }
    }

    fun neq(ax: u64, ay: u64, bx: u64, by: u64): bool {
        Point { x: ax, y: ay } != Point { x: bx, y: by }
    }

    fun cmp(ax: u64, ay: u64, bx: u64, by: u64): u64 {
        if (Point { x: ax, y: ay } == Point { x: bx, y: by }) { 10 } else { 42 }
    }
}

// RUN: execute 0x1::test::eq --args 3, 4, 3, 4
// CHECK: results: true

// RUN: execute 0x1::test::eq --args 9, 4, 3, 4
// CHECK: results: false

// RUN: execute 0x1::test::eq --args 3, 9, 3, 4
// CHECK: results: false

// RUN: execute 0x1::test::neq --args 3, 4, 3, 4
// CHECK: results: false

// RUN: execute 0x1::test::neq --args 3, 4, 3, 9
// CHECK: results: true

// RUN: execute 0x1::test::cmp --args 7, 8, 7, 8
// CHECK: results: 10

// RUN: execute 0x1::test::cmp --args 7, 8, 7, 9
// CHECK: results: 42
