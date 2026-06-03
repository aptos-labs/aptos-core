// RUN: publish
module 0x1::test_natives {
    native public fun u64_add(a: u64, b: u64): u64;
    native public fun u64_identity(x: u64): u64;
}
module 0x1::main {
    public fun via_add(a: u64, b: u64): u64 {
        0x1::test_natives::u64_add(a, b)
    }
    public fun via_identity(x: u64): u64 {
        0x1::test_natives::u64_identity(x)
    }
}

// RUN: execute 0x1::main::via_add --args 10, 20
// CHECK: results: 30

// RUN: execute 0x1::main::via_identity --args 99
// CHECK: results: 99
