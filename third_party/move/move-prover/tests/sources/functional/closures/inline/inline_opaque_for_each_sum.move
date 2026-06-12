// Exploits the dropped "no copy + ref capture" rule: a real `for_each` over a
// vector that sums elements through a `&mut`-capturing closure. This is the
// canonical use case that motivated dropping the rule.
//
// Source-level type-checking now passes (the closure has `copy + drop` and a
// `&mut` capture of `s`, which the dropped rule previously forbade).
//
// The HOF's body verification and the caller's spec proof both depend on a
// fold-of-ensures encoding at opaque call sites — the spec-side follow-up.
// Until that lands, both bodies use `pragma verify = false` and the test
// captures only the source-level admission of the pattern.
module 0x42::inline_opaque_for_each_sum {
    use std::vector;

    inline fun for_each<T>(v: &vector<T>, f: |&T| has copy + drop) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        }
    }
    spec for_each {
        pragma opaque;
        // Body has N calls to `f`; expressing the cumulative effect on f's
        // `&mut` captures requires a fold-of-ensures encoding (recursive spec
        // function over `write_of<f, j>`). That is the deferred follow-up.
        pragma verify = false;
        requires forall i in 0..len(v): !aborts_of<f>(v[i]);
        aborts_if false;
        ensures forall i in 0..len(v): ensures_of<f>(v[i]);
    }

    /// Sum via for_each with a `&mut`-capturing closure. This was rejected at
    /// the source level before the rule was dropped — the lambda's closure
    /// type has `copy + drop` (required by `for_each` calling it in a loop),
    /// but its capture of `&mut s` is a reference. The drop unblocks
    /// compilation; verification still requires the spec-side follow-up.
    fun sum(v: &vector<u64>): u64 {
        let s = 0;
        for_each(v, |e| s = s + *e spec {
            aborts_if s + e > MAX_U64;
            ensures s == old(s) + e;
        });
        s
    }
    spec sum {
        pragma verify = false;
        requires forall i in 0..len(v): v[i] <= 1000;
        requires len(v) <= 1000;
        aborts_if false;
        // What we'd like to prove (deferred):
        //   ensures result == spec_sum_of(v, len(v));
        // where spec_sum_of is the recursive vector sum. Cannot prove it yet
        // because the HOF's spec doesn't propagate the per-step effect on `s`
        // to a closed-form for the post-state.
    }

    /// Count elements satisfying a predicate via for_each. Demonstrates a
    /// different fold shape (counting, not summing).
    fun count_above(v: &vector<u64>, hi: u64): u64 {
        let c = 0;
        for_each(v, |e| {
            if (*e > hi) c = c + 1;
        } spec {
            aborts_if c == MAX_U64 && e > hi;
            ensures c == old(c) + (if (e > hi) 1 else 0);
        });
        c
    }
    spec count_above {
        pragma verify = false;
        requires len(v) <= MAX_U64;
        aborts_if false;
        ensures result <= len(v);
    }
}
