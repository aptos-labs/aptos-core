// O(n^2) double loop accumulating `i ^ j`.

// RUN: publish
module 0x1::nested_loop {
    public fun nested_loop(n: u64): u64 {
        let sum: u64 = 0;
        let i: u64 = 0;
        while (i < n) {
            let j: u64 = 0;
            while (j < n) {
                sum = sum + (i ^ j);
                j = j + 1;
            };
            i = i + 1;
        };
        sum
    }
}

// RUN: execute 0x1::nested_loop::nested_loop --args 0
// CHECK: results: 0
// RUN: execute 0x1::nested_loop::nested_loop --args 1
// CHECK: results: 0
// RUN: execute 0x1::nested_loop::nested_loop --args 2
// CHECK: results: 2
// RUN: execute 0x1::nested_loop::nested_loop --args 4
// CHECK: results: 24
// RUN: execute 0x1::nested_loop::nested_loop --args 10
// CHECK: results: 594
