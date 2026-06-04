// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    use std::vector;

    fun build(a: u64, b: u64): vector<u64> {
        let v = vector::empty<u64>();
        vector::push_back(&mut v, a);
        vector::push_back(&mut v, b);
        v
    }

    fun eq(a0: u64, a1: u64, b0: u64, b1: u64): bool {
        build(a0, a1) == build(b0, b1)
    }

    fun eq_len(a0: u64, b0: u64, b1: u64): bool {
        let a = vector::empty<u64>();
        vector::push_back(&mut a, a0);
        build(b0, b1) == a
    }

    fun cmp(a0: u64, a1: u64, b0: u64, b1: u64): u64 {
        if (build(a0, a1) == build(b0, b1)) { 10 } else { 42 }
    }
}

// RUN: execute 0x1::test::eq --args 1, 2, 1, 2
// CHECK: results: true

// RUN: execute 0x1::test::eq --args 1, 2, 1, 9
// CHECK: results: false

// RUN: execute 0x1::test::eq_len --args 1, 1, 2
// CHECK: results: false

// RUN: execute 0x1::test::cmp --args 5, 6, 5, 6
// CHECK: results: 10

// RUN: execute 0x1::test::cmp --args 5, 6, 5, 7
// CHECK: results: 42
