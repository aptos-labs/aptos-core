// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x99::capturing {
    fun add_u64(a: u64, b: u64): u64 {
        a + b
    }

    fun add3(a: u64, b: u64, c: u64): u64 {
        a + b + c
    }

    // Single leading capture: |y| x + y captures x; y supplied at call.
    fun adder(x: u64): u64 {
        let f: |u64|u64 has drop = |y| x + y;
        f(8)
    }

    // Closure created in one function, returned, and called in another.
    fun make_adder(x: u64): |u64|u64 has drop {
        |y| x + y
    }

    fun use_adder(a: u64, b: u64): u64 {
        let f = make_adder(a);
        f(b)
    }

    // Capture the second of two params, supply the first at call time
    // (mask 0b10): the provided arg lands at the leading callee slot.
    fun capture_second(b: u64): u64 {
        let f: |u64|u64 has drop = |a| add_u64(a, b);
        f(7)
    }

    // Capture at non-leading positions: a (pos 0) and c (pos 2) captured,
    // b (pos 1) supplied at call time -> exercises mask interleaving.
    fun capture_ends(a: u64, c: u64): u64 {
        let f: |u64|u64 has drop = |b| add3(a, b, c);
        f(100)
    }

    // Two captured values, one supplied later.
    fun capture_two(a: u64, b: u64): u64 {
        let f: |u64|u64 has drop = |c| add3(a, b, c);
        f(1)
    }

    // All params captured; the closure takes no arguments (mask 0b11).
    fun all_captured(a: u64, b: u64): u64 {
        let f: ||u64 has drop = || add_u64(a, b);
        f()
    }
}

// RUN: execute 0x99::capturing::adder --args 10
// CHECK: results: 18

// RUN: execute 0x99::capturing::use_adder --args 10, 8
// CHECK: results: 18

// RUN: execute 0x99::capturing::capture_second --args 35
// CHECK: results: 42

// RUN: execute 0x99::capturing::capture_ends --args 5, 7
// CHECK: results: 112

// RUN: execute 0x99::capturing::capture_two --args 20, 30
// CHECK: results: 51

// RUN: execute 0x99::capturing::all_captured --args 15, 27
// CHECK: results: 42
