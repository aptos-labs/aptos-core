// Unification of spec function specializations must require the attached
// lambda specs to correspond: for `result_of`/`ensures_of`, the substituted
// material comes from the attached spec when present, so lambdas with equal
// bodies but different (or absent) attached specs denote different
// specializations. Without this, the specialized recursion is built from
// whichever occurrence is processed first, making verification outcomes
// processing-order-dependent: with the guilty caller first, the innocent
// caller's accurate invariant fails against the poisoned recursion; with
// the innocent first, the guilty caller's wrong attached spec goes
// entirely unchecked.
module 0x42::unify_attached_specs {
    use std::vector;

    inline fun fold(v: &vector<u64>, init: u64, f: |u64, &u64| u64): u64 {
        let acc = init;
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            acc = f(acc, vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant acc == spec_fold(f, v, init, i); // error: for the `sum_attached_wrong` caller, the attached spec disagrees with the lambda's body
        };
        acc
    }

    /// Recursive definition of fold over the prefix `v[0..end]`.
    spec fun spec_fold(f: |u64, &u64| u64, v: vector<u64>, init: u64, end: u64): u64 {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    // Guilty caller (named to be processed first): the attached functional
    // spec disagrees with the lambda's body (it drops `e`), so the
    // substituted loop invariant relates `acc` to a constant recursion and
    // must fail — at this lambda's own expansion, and only there.
    fun sum_attached_wrong(v: &vector<u64>): u64 {
        fold(v, 0, |acc, e| acc + *e spec { ensures result == acc; })
    }
    spec sum_attached_wrong {
        pragma aborts_if_is_partial;
    }

    // Innocent caller: a spec-less lambda with the same body. Its
    // specialization derives `result_of` from the body, so the invariant is
    // the accurate recursion and verifies — regardless of the caller above
    // having been processed first.
    fun sum_plain(v: &vector<u64>): u64 {
        fold(v, 0, |acc, e| acc + *e)
    }
    spec sum_plain {
        pragma aborts_if_is_partial;
        ensures result == spec_fold(|acc, e| acc + e, v, 0, len(v));
    }
}
