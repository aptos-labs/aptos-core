// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun identity<A: drop, B: drop>(a: A, _b: B): A { a }

    fun caller<T: drop>(x: u64, y: T): u64 {
        identity<u64, T>(x, y)
    }

    public fun entry(y: u64): u64 {
        caller<u64>(42, y)
    }
}

// RUN: execute 0x1::test::entry --args 7
// CHECK: results: 42
