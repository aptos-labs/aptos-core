// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x99::closure_as_arg {
    fun add_u64(a: u64, b: u64): u64 {
        a + b
    }

    // Closure passed as a parameter and invoked by the callee.
    fun apply(f: |u64|u64, x: u64): u64 {
        f(x)
    }

    // Pack a capturing closure, hand it to `apply`, which invokes it.
    fun run(y: u64, x: u64): u64 {
        let f: |u64|u64 has drop = |arg| add_u64(y, arg);
        apply(f, x)
    }
}

// RUN: execute 0x99::closure_as_arg::run --args 10, 32
// CHECK: results: 42
