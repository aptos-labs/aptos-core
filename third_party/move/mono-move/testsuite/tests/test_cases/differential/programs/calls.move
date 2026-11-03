// Each loop performs `n` calls that increment an accumulator and returns `n`.
// The call kinds exercise different dispatch paths and reentrancy checks:
// - same_module: an exempt call.
// - cross_module: a checked call.
// - closure: a closure call, which is always checked.

// RUN: publish
module 0x1::calls_helper {
    public fun add_one(x: u64): u64 {
        x + 1
    }
}

module 0x1::calls {
    use 0x1::calls_helper;

    fun add_one(x: u64): u64 {
        x + 1
    }

    public fun same_module(n: u64): u64 {
        let acc = 0;
        let i = 0;
        while (i < n) {
            acc = add_one(acc);
            i += 1;
        };
        acc
    }

    public fun cross_module(n: u64): u64 {
        let acc = 0;
        let i = 0;
        while (i < n) {
            acc = calls_helper::add_one(acc);
            i += 1;
        };
        acc
    }

    public fun closure(n: u64): u64 {
        let f: |u64|u64 has copy + drop = |x: u64| x + 1;
        let acc = 0;
        let i = 0;
        while (i < n) {
            acc = f(acc);
            i += 1;
        };
        acc
    }
}

// RUN: execute 0x1::calls::same_module --args 0
// CHECK: results: 0
// RUN: execute 0x1::calls::same_module --args 1000
// CHECK: results: 1000

// RUN: execute 0x1::calls::cross_module --args 0
// CHECK: results: 0
// RUN: execute 0x1::calls::cross_module --args 1000
// CHECK: results: 1000

// RUN: execute 0x1::calls::closure --args 0
// CHECK: results: 0
// RUN: execute 0x1::calls::closure --args 1000
// CHECK: results: 1000
