// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun pick_second<A: drop, B: drop>(_a: A, b: B): B { b }

    public fun entry(seed: u64): u64 {
        pick_second<u64, u64>(99, seed)
    }
}

// RUN: execute 0x1::test::entry --args 42
// CHECK: results: 42
