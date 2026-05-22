// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun pick_first<A: drop, B: drop>(a: A, _b: B): A { a }

    public fun entry(seed: u64): u64 {
        pick_first<u64, u64>(seed, 99)
    }
}

// RUN: execute 0x1::test::entry --args 42
// CHECK: results: 42
