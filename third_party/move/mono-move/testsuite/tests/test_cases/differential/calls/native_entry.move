// Executing a native function directly, as the function to run. V1 fails with an internal
// error before the native runs; MonoMove refuses to load a native as code. The first step
// calls the same native through a wrapper, which works on both VMs.

// RUN: publish
module 0x1::test_natives {
    native public fun u64_add(lhs: u64, rhs: u64): u64;
}
module 0x1::wrapper {
    public fun add(lhs: u64, rhs: u64): u64 {
        0x1::test_natives::u64_add(lhs, rhs)
    }
}

// RUN: execute 0x1::wrapper::add --args 7, 35
// CHECK: results: 42

// RUN: execute 0x1::test_natives::u64_add --args 7, 35
// CHECK-V1: error: VMError with status VERIFIER_INVARIANT_VIOLATION at location UNDEFINED and message local index out of bounds: got 0, len: 0
// CHECK-V2: error: failed to load function: Function 0x1::test_natives::u64_add is a native and cannot be loaded as code
