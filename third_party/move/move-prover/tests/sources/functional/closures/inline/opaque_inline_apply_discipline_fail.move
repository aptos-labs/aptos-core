// A callee's `ensures_of` claim over a fun parameter is provable exactly when
// the body applies the parameter once with the claimed arguments: at each
// application the fun value advances to the `result_of` successor, and the
// claim's post value must equal it. Bodies applying the parameter zero or two
// times, or on other arguments, cannot verify a single-application claim —
// which keeps the call-site assumption (one application constrains the
// havoced capture) coherent with what the body actually did.
module 0x42::opaque_inline_apply_discipline_fail {

    inline fun claim_without_apply(f: |u64| has copy + drop) {
        let _ = f;
    }
    spec claim_without_apply {
        pragma opaque;
        ensures ensures_of<f>(1); // error: post-condition does not hold
    }

    inline fun claim_once_apply_twice(f: |u64| has copy + drop) {
        f(1);
        f(1);
    }
    spec claim_once_apply_twice {
        pragma opaque;
        ensures ensures_of<f>(1); // error: post-condition does not hold
    }

    inline fun claim_wrong_arg(f: |u64| has copy + drop) {
        f(2)
    }
    spec claim_wrong_arg {
        pragma opaque;
        ensures ensures_of<f>(1); // error: post-condition does not hold
    }

    /// A mutating caller making the fun type carrying; without a carrying
    /// variant the successor machinery is not exercised.
    fun make_carrying(): u64 {
        let x = 0;
        claim_without_apply(|i| x = x + i spec { ensures x == old(x) + i; });
        x
    }
}
