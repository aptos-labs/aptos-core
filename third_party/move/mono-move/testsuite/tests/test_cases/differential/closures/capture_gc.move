// Capturing closures under collection. `pack_gc_once` allocates a dead vector (in `junk`),
// the captured vector, the closure, and then the closure's captured data. With a heap of 120
// to 159 bytes only the captured-data allocation collects, at a point where no local holds the
// new closure yet; at 160 bytes everything fits.

// RUN: publish
module 0x42::capture_gc {
    use std::vector;

    struct Holder has drop { f: |u64|u64 has drop }

    fun junk(value: u64): u64 {
        let garbage = vector[value, value, value];
        vector::length(&garbage)
    }

    fun sum_plus(values: vector<u64>, offset: u64, arg: u64): u64 {
        *vector::borrow(&values, 0) + vector::length(&values) + offset + arg
    }

    public fun pack_gc_once(): u64 {
        let count = junk(1);
        let values = vector[10, 20, 30];
        let func = |arg| sum_plus(values, 0, arg);
        func(count)
    }

    // At heap 304, allocating the captured data collects on every iteration from the third on.
    public fun pack_under_pressure(count: u64): u64 {
        let acc = 0;
        let index = 0;
        while (index < count) {
            let values = vector[index, index + 1];
            let func = |arg| sum_plus(values, index, arg);
            acc = acc + func(1);
            index = index + 1;
        };
        acc
    }

    // Position-weighted digest, so every element of the captured vector matters.
    fun weigh(values: vector<u64>, arg: u64): u64 {
        *vector::borrow(&values, 0) * 10000 + *vector::borrow(&values, 1) * 100 + *vector::borrow(&values, 2) + arg * 1000000
    }

    // Builds the vector at runtime and captures it. Once `make` returns, the vector is
    // reachable only through the closure.
    fun make(first: u64, second: u64): |u64|u64 has drop {
        let values = vector::empty<u64>();
        vector::push_back(&mut values, first);
        vector::push_back(&mut values, second);
        vector::push_back(&mut values, first + second);
        |arg| weigh(values, arg)
    }

    // The closure is held in a local across the first collection and in a struct field
    // across the second; its captured vector must survive both.
    public fun live_across_gc(first: u64, second: u64): u64 {
        let func = make(first, second);
        0x0::test_utils::force_gc();
        let holder = Holder { f: func };
        0x0::test_utils::force_gc();
        let Holder { f: func } = holder;
        func(1)
    }
}

// RUN: execute 0x42::capture_gc::pack_gc_once --heap-size 120
// CHECK: results: 16
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::capture_gc::pack_gc_once --heap-size 159
// CHECK: results: 16
// CHECK-GC-COUNT: 1

// RUN: execute 0x42::capture_gc::pack_gc_once --heap-size 160
// CHECK: results: 16
// CHECK-GC-COUNT: 0

// RUN: execute 0x42::capture_gc::pack_under_pressure --args 10 --heap-size 304
// CHECK: results: 120
// CHECK-GC-COUNT: 8

// RUN: execute 0x42::capture_gc::live_across_gc --args 4, 5
// CHECK: results: 1040509
// CHECK-GC-COUNT: 2
