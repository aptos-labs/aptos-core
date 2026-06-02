// RUN: publish
module 0x1::test_natives {
    native public fun u64_add(a: u64, b: u64): u64;
}
module 0x1::main {
    public fun wrap(a: u64, b: u64): u64 {
        0x1::test_natives::u64_add(a, b)
    }
}

// RUN: execute 0x1::main::wrap --args 7, 35
// CHECK: results: 42

// RUN: execute 0x1::main::wrap --args 0, 0
// CHECK: results: 0

// RUN: execute 0x1::main::wrap --args 18446744073709551615, 1
// CHECK: aborted: code 1
