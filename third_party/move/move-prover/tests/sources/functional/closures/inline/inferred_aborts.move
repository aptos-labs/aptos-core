// flag: --infer-lambda-specs
//
// The lambda's inferred `aborts_if y + 1 > MAX_U64` propagates through the
// opaque callee's `aborts_if aborts_of<f>(x);` to constrain the caller's
// abort condition.
module 0x42::inferred_aborts {

    inline fun guarded(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec guarded {
        pragma opaque;
        aborts_if aborts_of<f>(x);
        ensures ensures_of<f>(x, result);
    }

    fun test(x: u64): u64 {
        guarded(|y| y + 1, x)
    }
    spec test {
        aborts_if x + 1 > MAX_U64;
        ensures result == x + 1;
    }
}
