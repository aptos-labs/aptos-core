// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    struct Point has copy, drop {
        x: u64,
        y: u64,
    }

    fun eq_ref(ax: u64, ay: u64, bx: u64, by: u64): bool {
        let a = Point { x: ax, y: ay };
        let b = Point { x: bx, y: by };
        &a == &b
    }

    fun cmp_ref(ax: u64, ay: u64, bx: u64, by: u64): u64 {
        let a = Point { x: ax, y: ay };
        let b = Point { x: bx, y: by };
        if (&a == &b) { 10 } else { 42 }
    }
}

// RUN: execute 0x1::test::eq_ref --args 3, 4, 3, 4
// CHECK: results: true

// RUN: execute 0x1::test::eq_ref --args 3, 4, 3, 9
// CHECK: results: false

// RUN: execute 0x1::test::cmp_ref --args 7, 8, 7, 8
// CHECK: results: 10

// RUN: execute 0x1::test::cmp_ref --args 7, 8, 9, 8
// CHECK: results: 42
