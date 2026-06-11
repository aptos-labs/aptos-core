// RUN: publish
module 0x42::vec_sum_gc {
    use std::vector;

    public fun sum_with_gc(n: u64): u64 {
        let v = vector::empty<u64>();
        let i = 0;
        while (i < n) {
            vector::push_back(&mut v, i);
            i = i + 1;
        };
        0x0::test_utils::force_gc();
        let acc = 0;
        let len = vector::length(&v);
        while (len > 0) {
            acc = acc + vector::pop_back(&mut v);
            len = len - 1;
        };
        acc
    }
}

// RUN: execute 0x42::vec_sum_gc::sum_with_gc --args 0
// CHECK: results: 0
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::vec_sum_gc::sum_with_gc --args 10
// CHECK: results: 45
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::vec_sum_gc::sum_with_gc --args 100
// CHECK: results: 4950
// CHECK-GC-COUNT: 1
