// Dead-store elimination and copy propagation must preserve values across
// branch joins, loop iterations, and writes through references.

// RUN: publish
module 0x42::dead_store_joins {
    // Both branches overwrite the initial value; the result uses the taken branch.
    fun overwrite_join(cond: bool): u64 {
        let x = 1;
        if (cond) { x = 2; } else { x = 3; };
        x
    }

    // Loop-carried overwrite.
    fun loop_carry(n: u64): u64 {
        let x = 0;
        let i = 0;
        while (i < n) { x = i * 2; i = i + 1; };
        x
    }

    // The assignment of 99 is overwritten before any read.
    fun store_after_read(): u64 {
        let x = 5;
        let y = x + 1;
        x = 99;
        x = 7;
        x + y
    }

    // Copy chain: every link must resolve to the original value.
    fun copy_chain(seed: u64): u64 {
        let a = seed;
        let b = a;
        let c = b;
        let d = c;
        d + 1
    }

    // Write in both arms, read in a nested branch.
    fun nested_join(cond: bool, x: u64): u64 {
        let v = x;
        if (cond) {
            v = v + 10;
            if (x > 3) { v = v + 100; };
        } else {
            v = v * 2;
        };
        v
    }

    // The call updates x through a reference; y must retain the copied value.
    fun borrow_kills_prop(): u64 {
        let x = 1;
        let y = x;
        bump(&mut x);
        x * 10 + y
    }

    fun bump(r: &mut u64) { *r = *r + 1; }
}

// RUN: execute 0x42::dead_store_joins::overwrite_join --args true
// CHECK: results: 2

// RUN: execute 0x42::dead_store_joins::overwrite_join --args false
// CHECK: results: 3

// RUN: execute 0x42::dead_store_joins::loop_carry --args 4
// CHECK: results: 6

// RUN: execute 0x42::dead_store_joins::loop_carry --args 0
// CHECK: results: 0

// RUN: execute 0x42::dead_store_joins::store_after_read
// CHECK: results: 13

// RUN: execute 0x42::dead_store_joins::copy_chain --args 41
// CHECK: results: 42

// RUN: execute 0x42::dead_store_joins::nested_join --args true, 5
// CHECK: results: 115

// RUN: execute 0x42::dead_store_joins::nested_join --args true, 1
// CHECK: results: 11

// RUN: execute 0x42::dead_store_joins::nested_join --args false, 5
// CHECK: results: 10

// RUN: execute 0x42::dead_store_joins::borrow_kills_prop
// CHECK: results: 21
