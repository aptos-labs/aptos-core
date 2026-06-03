// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun innermost<T: drop>(x: T): T { x }

    fun middle<T: drop>(x: T): T { innermost<T>(x) }

    fun outer<T: drop>(x: T): T { middle<T>(x) }

    public fun entry(x: u64): u64 {
        outer<u64>(x)
    }
}

// RUN: execute 0x1::test::entry --args 42
// CHECK: results: 42
