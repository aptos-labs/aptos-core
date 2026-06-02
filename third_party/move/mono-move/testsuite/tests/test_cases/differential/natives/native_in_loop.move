// RUN: publish
module 0x1::test_natives {
    native public fun u64_add(a: u64, b: u64): u64;
}
module 0x1::main {
    // Repeated native call inside a loop body, with non-native
    // bytecode (arithmetic, branches) interleaved.
    public fun sum_via_native(n: u64): u64 {
        let i = 0;
        let acc = 0;
        while (i < n) {
            acc = 0x1::test_natives::u64_add(acc, i);
            i = i + 1;
        };
        acc
    }
}

// RUN: execute 0x1::main::sum_via_native --args 0
// CHECK: results: 0

// RUN: execute 0x1::main::sum_via_native --args 1
// CHECK: results: 0

// RUN: execute 0x1::main::sum_via_native --args 10
// CHECK: results: 45
