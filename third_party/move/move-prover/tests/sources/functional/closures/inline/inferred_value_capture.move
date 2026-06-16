// A lambda without an explicit spec block: spec inference synthesizes
// `ensures result == y + c; aborts_if y + c > MAX_U64;` from the body, so the
// behavioral predicate `ensures_of<f>(x, result)` in `apply`'s opaque spec
// gives the caller enough information to prove `result == x + c`.
module 0x42::inferred_value_capture {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    fun test_value_capture(x: u64, c: u64): u64 {
        apply(|y| y + c, x)
    }
    spec test_value_capture {
        requires x < (1 << 32) && c < (1 << 32);
        ensures result == x + c;
    }

    /// Two distinct captures in one call.
    inline fun apply2(f: |u64| u64, g: |u64| u64, x: u64): u64 {
        g(f(x))
    }
    spec apply2 {
        pragma opaque;
        ensures exists m: u64: ensures_of<f>(x, m) && ensures_of<g>(m, result);
    }

    fun test_two_lambdas(x: u64, a: u64, b: u64): u64 {
        apply2(|y| y + a, |y| y + b, x)
    }
    spec test_two_lambdas {
        requires x < (1 << 32) && a < (1 << 32) && b < (1 << 32);
        ensures result == x + a + b;
    }
}
