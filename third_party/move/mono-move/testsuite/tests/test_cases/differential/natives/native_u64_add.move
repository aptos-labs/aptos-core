// A call to a test native, including its overflow abort. The golden pins how the call
// lowers and the entry gas of its block. The printed lowering calls the native by name,
// while execution calls it directly; the gas does not depend on that, so the printed
// value is the one execution charges.

// RUN: publish --print(stackless,micro-ops)
module 0x1::test_natives {
    native public fun u64_add(lhs: u64, rhs: u64): u64;
}
module 0x1::main {
    public fun wrap(lhs: u64, rhs: u64): u64 {
        0x1::test_natives::u64_add(lhs, rhs)
    }
}

// RUN: execute 0x1::main::wrap --args 7, 35
// CHECK: results: 42

// RUN: execute 0x1::main::wrap --args 18446744073709551615, 1
// CHECK: aborted: code 1 in 0x1::test_natives
