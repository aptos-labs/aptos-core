// A function value in a struct field that follows a `u8` field: a vector of such structs
// built with a collection after each push, and a copied struct that keeps its own function
// value after the original's is overwritten.

// RUN: publish
module 0x42::cis {
    use std::vector;

    struct H has copy, drop, store { n: u8, f: |u64|u64 has copy + drop + store, m: u64 }

    public fun add(lhs: u64, rhs: u64): u64 { lhs + rhs }
    public fun scale(values: vector<u64>, arg: u64): u64 { arg * vector::length(&values) }

    fun make(index: u8, base: u64): H {
        let values = vector[base, base, base];
        if (index % 2 == 0) H { n: index, f: |arg| add(base, arg), m: base * 10 }
        else H { n: index, f: |arg| scale(values, arg), m: base * 100 }
    }

    public fun vec_of_structs(count: u64, arg: u64): vector<u64> {
        let hs = vector[];
        let index = 0;
        while (index < count) {
            vector::push_back(&mut hs, make((index as u8), index + 1));
            0x0::test_utils::force_gc();
            index = index + 1;
        };
        let out = vector[];
        vector::for_each_ref(&hs, |holder| {
            let holder: &H = holder;
            vector::push_back(&mut out, (holder.n as u64) + (holder.f)(arg) + holder.m)
        });
        out
    }

    public fun copy_then_overwrite(arg: u64): u64 {
        let original = make(0, 5);
        let copied = original;
        original.f = |value| add(1000, value);
        0x0::test_utils::force_gc();
        (original.f)(arg) * 10000 + (copied.f)(arg)
    }
}

// RUN: execute 0x42::cis::vec_of_structs --args 5, 2 --heap-size 1024
// CHECK: results: [13, 207, 37, 409, 61]
// CHECK-GC-COUNT: 6

// RUN: execute 0x42::cis::copy_then_overwrite --args 3 --heap-size 512
// CHECK: results: 10030008
// CHECK-GC-COUNT: 1
