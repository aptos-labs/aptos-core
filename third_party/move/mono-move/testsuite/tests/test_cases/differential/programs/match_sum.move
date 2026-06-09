// O(n) loop with a 4-arm match (wide-diamond CFG).

// RUN: publish
module 0x1::match_sum {
    public fun match_sum(n: u64): u64 {
        let sum: u64 = 0;
        let i: u64 = 0;
        while (i < n) {
            let r = i % 4;
            if (r == 0) {
                sum = sum + 10;
            } else if (r == 1) {
                sum = sum + 20;
            } else if (r == 2) {
                sum = sum + 30;
            } else {
                sum = sum + 40;
            };
            i = i + 1;
        };
        sum
    }
}

// RUN: execute 0x1::match_sum::match_sum --args 0
// CHECK: results: 0
// RUN: execute 0x1::match_sum::match_sum --args 1
// CHECK: results: 10
// RUN: execute 0x1::match_sum::match_sum --args 2
// CHECK: results: 30
// RUN: execute 0x1::match_sum::match_sum --args 3
// CHECK: results: 60
// RUN: execute 0x1::match_sum::match_sum --args 4
// CHECK: results: 100
// RUN: execute 0x1::match_sum::match_sum --args 8
// CHECK: results: 200
// RUN: execute 0x1::match_sum::match_sum --args 100
// CHECK: results: 2500
