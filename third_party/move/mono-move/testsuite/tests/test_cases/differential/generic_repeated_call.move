// RUN: publish --print(bytecode,stackless,micro-ops)
module 0x1::test {
    fun identity<T: drop>(x: T): T { x }

    public fun entry(): u64 {
        let a = identity<u64>(10);
        let b = identity<u64>(20);
        let c = identity<u64>(12);
        a + b + c
    }
}

// RUN: execute 0x1::test::entry
// CHECK: results: 42
