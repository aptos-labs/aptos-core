// The body of an inline function with a function-level spec is verified
// against the spec. Here the body returns `x + 1` but the spec claims the
// result is `x + 2`, so verification of the inline function's spec must fail.
module 0x42::TestInlineFunSpecErr {

    spec module {
        pragma verify = true;
    }

    public inline fun add_one_wrong_spec(x: u64): u64 {
        x + 1
    }
    spec add_one_wrong_spec {
        aborts_if x == 0xFFFFFFFFFFFFFFFF;
        ensures result == x + 2;
    }
}
