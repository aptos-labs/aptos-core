// RUN: publish
module 0x1::test_natives {
    native public fun u64_add(a: u64, b: u64): u64;
}
module 0x1::main {
    // Native dispatched from different branches of a conditional.
    public fun branch(cond: u64, a: u64, b: u64): u64 {
        if (cond < 1) {
            0x1::test_natives::u64_add(a, b)
        } else {
            0x1::test_natives::u64_add(b, a)
        }
    }
}

// RUN: execute 0x1::main::branch --args 0, 10, 5
// CHECK: results: 15

// RUN: execute 0x1::main::branch --args 1, 10, 5
// CHECK: results: 15
