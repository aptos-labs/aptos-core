// A function with a Move body under the name of a native (`test_natives::u64_add`). V1
// runs the body; MonoMove deliberately refuses to load it rather than run the native in
// its place.

// RUN: publish
module 0x1::test_natives {
    public fun u64_add(left: u64, right: u64): u64 {
        left + right + 1
    }
}

// RUN: execute 0x1::test_natives::u64_add --args 2, 3
// CHECK-V1: results: 6
// CHECK-V2: error: failed to load function: Native registered for 0x1::test_natives::u64_add, which has a Move body
