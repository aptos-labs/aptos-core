// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun identity<T>(x: T): T { x }

    public fun call_identity_u64(x: u64): u64 {
        identity<u64>(x)
    }
}

// RUN: execute 0x1::test::call_identity_u64 --args 42
// CHECK: results: 42
