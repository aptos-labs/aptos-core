// When the user writes a spec block on the lambda, inference must not append
// or overwrite. The user's spec is what callers see; the body verifies against
// it as today.
module 0x42::inferred_user_spec_preserved {

    inline fun apply(f: |u64| u64, x: u64): u64 {
        f(x)
    }
    spec apply {
        pragma opaque;
        ensures ensures_of<f>(x, result);
    }

    /// User wrote a spec; lambda body matches it. Caller proves through the
    /// user spec; if inference appended contradictory conditions, the lifted
    /// lambda's body verification would fail.
    fun test_user_spec_drives(x: u64, c: u64): u64 {
        apply(|y| y + c spec { ensures result == y + c; }, x)
    }
    spec test_user_spec_drives {
        requires x < (1 << 32) && c < (1 << 32);
        ensures result == x + c;
    }
}
