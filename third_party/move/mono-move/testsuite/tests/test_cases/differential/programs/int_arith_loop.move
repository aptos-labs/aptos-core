// Each iteration runs 30 rounds of `acc = (acc * 31 + 17) % 1000003`. u64_loop
// lowers to specialized arithmetic ops, i64_loop to unspecialized — same work,
// different encoding.

// RUN: publish
module 0x1::int_arith_loop {
    public fun u64_loop(iters: u64): u64 {
        let acc: u64 = 1;
        let i: u64 = 0;
        while (i < iters) {
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            i = i + 1;
        };
        acc
    }

    public fun i64_loop(iters: u64): i64 {
        let acc: i64 = 1;
        let i: u64 = 0;
        while (i < iters) {
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            acc = ((acc * 31) + 17) % 1000003;
            i = i + 1;
        };
        acc
    }
}

// RUN: execute 0x1::int_arith_loop::u64_loop --args 1
// CHECK: results: 97933
// RUN: execute 0x1::int_arith_loop::i64_loop --args 1
// CHECK: results: 97933
// RUN: execute 0x1::int_arith_loop::u64_loop --args 5
// CHECK: results: 542676
// RUN: execute 0x1::int_arith_loop::i64_loop --args 5
// CHECK: results: 542676
// RUN: execute 0x1::int_arith_loop::u64_loop --args 100
// CHECK: results: 226958
// RUN: execute 0x1::int_arith_loop::i64_loop --args 100
// CHECK: results: 226958
