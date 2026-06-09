// RUN: publish
module 0x1::test_natives {
    native public fun u64_add(a: u64, b: u64): u64;
    native public fun u64_identity(x: u64): u64;
}
module 0x1::main {
    // Native calls mixed with non-native arithmetic and a Move function call.
    fun helper(x: u64): u64 { x * 2 }

    public fun mix(a: u64, b: u64): u64 {
        let x = 0x1::test_natives::u64_identity(a);
        let y = b + 1;
        let z = 0x1::test_natives::u64_add(x, y);
        let w = helper(z);
        0x1::test_natives::u64_add(w, 3)
    }
}

// RUN: execute 0x1::main::mix --args 5, 7
// CHECK: results: 29

// RUN: execute 0x1::main::mix --args 0, 0
// CHECK: results: 5
