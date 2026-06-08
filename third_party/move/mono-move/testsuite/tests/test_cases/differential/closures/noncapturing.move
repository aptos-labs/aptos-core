// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x99::noncapturing {
    fun identity(x: u64): u64 {
        x
    }

    fun double(x: u64): u64 {
        x + x
    }

    // Bind a top-level function to a (non-capturing) closure, then call it.
    fun call_identity(n: u64): u64 {
        let f: |u64|u64 has drop = identity;
        f(n)
    }

    // Non-capturing closure stored in a local, called twice.
    fun call_double_twice(n: u64): u64 {
        let f: |u64|u64 has copy + drop = double;
        f(n) + f(n)
    }
}

// RUN: execute 0x99::noncapturing::call_identity --args 42
// CHECK: results: 42

// RUN: execute 0x99::noncapturing::call_double_twice --args 21
// CHECK: results: 84
