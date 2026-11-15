// RUN: publish --print(stackless)
module 0x42::closure_ret_type_discovery {
    struct S<T> has copy, drop { x: T }

    fun pair<T: copy>(x: T): (S<T>, S<T>) {
        (S { x }, S { x })
    }

    // In this function, `S<u64>` appears only in the closure's result types.
    public fun test(v: u64): bool {
        let f: |u64|(S<u64>, S<u64>) has drop = |x| pair(x);
        let (a, b) = f(v);
        a == b
    }
}

// RUN: execute 0x42::closure_ret_type_discovery::test --args 7
// CHECK: results: true
