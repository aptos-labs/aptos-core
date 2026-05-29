// Function-level specs on inline functions are not supported when the
// function has a parameter of function type, because the lambda is inlined
// at the call site rather than passed as a value the spec can refer to.
module 0x42::M {
    public inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        ensures result == f(x);
    }
}
