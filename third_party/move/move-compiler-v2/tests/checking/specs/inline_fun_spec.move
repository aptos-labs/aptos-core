module 0x42::M {
    // Specs on inline functions without function-typed parameters are allowed.
    public inline fun f(): u64 {
        42
    }
    spec f {
        aborts_if false;
        ensures result == 42;
    }

    // Specs on inline functions with function-typed parameters are an error.
    public inline fun g(h: |u64| u64, x: u64): u64 {
        h(x)
    }
    spec g {
        aborts_if false;
    }
}
