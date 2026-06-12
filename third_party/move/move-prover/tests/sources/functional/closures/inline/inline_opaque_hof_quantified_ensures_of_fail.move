// Regression: a HOF whose opaque spec contains a quantified `ensures_of<f>`
// (e.g. `forall i: ensures_of<f>(v[i])`) over a single captured-mut location
// is unsound at the call site. The quantifier semantically generates N
// constraints on the SAME havoced post-state of f's captures; for inputs that
// make those constraints distinct, the assumption is inconsistent and the
// caller's post-condition is provable vacuously.
//
// Before the fix, the caller below verified its deliberately false
// `ensures len(v) >= 2 ==> v[0] == v[1]` because the only inputs that
// falsified it (v with two distinct elements) were exactly the ones where
// the over-constraint became inconsistent.
//
// The fix lives at `spec_instrumentation.rs` in
// `rewrite_behavior_preds_for_captured_muts`: a pre-scan rejects quantified
// `ensures_of` over closures that have captured `&mut` locals at the call site.
module 0x42::inline_opaque_hof_quantified_ensures_of_fail {
    use std::vector;

    inline fun for_each_ref<T>(v: &vector<T>, f: |&T| has copy + drop) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        }
    }
    spec for_each_ref {
        pragma opaque;
        requires forall i in 0..len(v): !aborts_of<f>(v[i]);
        aborts_if false;
        ensures forall i in 0..len(v): ensures_of<f>(v[i]);
    }

    /// Caller with a `&mut`-capturing lambda: the closure-checker admits the
    /// copy + ref capture (post the over-conservative rule's removal), but the
    /// spec-instrumentation pre-scan rejects the combination of the HOF's
    /// quantified `ensures_of` and the captured `&mut s`.
    fun unsound_call(v: &vector<u64>): bool {
        let s = 0;
        for_each_ref(v, |e| s = s + *e spec { // error: quantified `ensures_of`
            aborts_if s + e > MAX_U64;
            ensures s == old(s) + e;
        });
        s == s
    }
    spec unsound_call {
        requires forall i in 0..len(v): v[i] < (1 << 32);
        requires len(v) < 100;
        aborts_if false;
        // Semantically false for v = [1, 2]. Without the fix, the prover
        // accepted this vacuously.
        ensures len(v) >= 2 ==> v[0] == v[1];
    }
}
