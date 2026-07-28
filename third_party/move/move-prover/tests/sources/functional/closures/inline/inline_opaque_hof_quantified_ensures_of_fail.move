// A loop-invoking HOF (`for_each_ref` style) with a `&mut`-capturing lambda —
// the canonical accumulator pattern — instantiated against a (trusted)
// quantified spec is rejected: `forall i: ensures_of<f>(v[i])` would generate
// N constraints on the SINGLE havoced post-state of the capture; for inputs
// making those constraints distinct the assumption is inconsistent and the
// caller's post-condition would be provable vacuously (before the guard, the
// deliberately false `ensures len(v) >= 2 ==> v[0] == v[1]` below verified
// for exactly the inputs that falsified it). Expressing N sequential
// applications requires `fun_post_of` chains (see issue #20273).
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
        pragma verify = false;
        requires forall i in 0..len(v): !aborts_of<f>(v[i]);
        aborts_if false;
        ensures forall i in 0..len(v): ensures_of<f>(v[i]);
    }

    /// Caller with a `&mut`-capturing lambda: the quantified `ensures_of` in
    /// the callee's spec is rejected at the call site.
    fun unsound_call(v: &vector<u64>): bool {
        let s = 0;
        for_each_ref(v, |e| s = s + *e spec { // error: a quantified `ensures_of` or `fun_post_of` over a function value carrying mutable reference captures would over-constrain its single post value
            aborts_if s + e > MAX_U64;
            ensures s == old(s) + e;
        });
        s == s
    }
    spec unsound_call {
        requires forall i in 0..len(v): v[i] < (1 << 32);
        requires len(v) < 100;
        aborts_if false;
        // Semantically false for v = [1, 2]. Without the guard, the prover
        // accepted this vacuously.
        ensures len(v) >= 2 ==> v[0] == v[1];
    }
}
