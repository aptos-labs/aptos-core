// End-to-end test of `for_each_mut`: per-element `&mut T` transformation via
// an opaque inline HOF. The lambda receives `&mut Element` directly, so it
// can mutate each slot without `&mut` captures of outer locals (which would
// be incompatible with the `copy` ability required to call `f` in a loop).
module 0x42::inline_opaque_hof_for_each_mut {
    use std::vector;

    inline fun for_each_mut<T>(v: &mut vector<T>, f: |&mut T| has copy + drop) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow_mut(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant len(v) == len(old(v));
            invariant n == len(v);
            invariant forall j in 0..i: ensures_of<f>(old(v)[j], v[j]);
            invariant forall j in 0..i: !aborts_of<f>(old(v)[j]);
            invariant forall j in i..n: v[j] == old(v)[j];
        };
    }
    spec for_each_mut {
        pragma opaque;
        requires forall i in 0..len(v): !aborts_of<f>(v[i]);
        aborts_if false;
        ensures len(v) == len(old(v));
        ensures forall i in 0..len(v): ensures_of<f>(old(v)[i], v[i]);
    }

    /// Caller: increment every element. The lambda's spec relates pre and
    /// post states of the `&mut u64` slot. The caller's `requires` discharges
    /// the per-element abort condition.
    fun increment_all(v: &mut vector<u64>) {
        for_each_mut(v, |e| {
            *e = *e + 1
        } spec {
            aborts_if e == MAX_U64;
            ensures e == old(e) + 1;
        });
    }
    spec increment_all {
        requires forall i in 0..len(v): v[i] < MAX_U64;
        aborts_if false;
        ensures len(v) == len(old(v));
        ensures forall i in 0..len(v): v[i] == old(v)[i] + 1;
    }

    /// Caller: scale every element by a constant factor. Demonstrates the
    /// HOF's spec extending a relational per-element transformation to the
    /// whole vector.
    fun scale_all(v: &mut vector<u64>, k: u64) {
        for_each_mut(v, |e| {
            *e = *e * k
        } spec {
            aborts_if e * k > MAX_U64;
            ensures e == old(e) * k;
        });
    }
    spec scale_all {
        requires forall i in 0..len(v): v[i] * k <= MAX_U64;
        aborts_if false;
        ensures len(v) == len(old(v));
        ensures forall i in 0..len(v): v[i] == old(v)[i] * k;
    }

    /// Caller: clamp every element to a maximum. Uses control flow in the
    /// lambda body; the spec relates pre and post via `if`.
    fun clamp_all(v: &mut vector<u64>, cap: u64) {
        for_each_mut(v, |e| {
            if (*e > cap) *e = cap
        } spec {
            aborts_if false;
            ensures e == if (old(e) > cap) cap else old(e);
        });
    }
    spec clamp_all {
        aborts_if false;
        ensures len(v) == len(old(v));
        ensures forall i in 0..len(v):
            v[i] == if (old(v)[i] > cap) cap else old(v)[i];
        ensures forall i in 0..len(v): v[i] <= cap || v[i] == old(v)[i];
    }
}
