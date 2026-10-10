// The `_mut` dual-form counterpart of `folds_of_wrapper.move`: a wrapper
// HOF forwarding to an inner `for_each_mut`-style inline HOF through a
// `borrow_kv_mut` projection prelude. The inner invariants use the
// point-wise dual-form `ensures_of`/`aborts_of` set (no `folds_of`; see
// the note in `std::vector` — `folds_of` does not support `&mut`
// parameters), which composes through the forwarder via the derived
// projection places (writes through the returned references become field
// updates of the element) and the deferred predicates over the wrapper's
// parameter. Additionally, the same wrapper is expanded twice in one
// caller, exercising the per-expansion freshening of any anchored
// material.
module 0x42::folds_of_wrapper_mut {
    use std::vector;

    inline fun each_mut<T>(v: &mut vector<T>, f: |&mut T|) {
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

    struct Entry has copy, drop, store {
        key: u64,
        value: u64,
    }

    fun borrow_kv_mut(self: &mut Entry): (&mut u64, &mut u64) {
        (&mut self.key, &mut self.value)
    }

    inline fun each_kv_mut(entries: &mut vector<Entry>, f: |&u64, &mut u64|) {
        each_mut(entries, |elem| {
            let (key, value) = elem.borrow_kv_mut();
            f(key, value)
        });
    }

    /// The concrete lambda's effect on the projected `&mut` reference
    /// becomes a field update of the element; the dual-form `ensures_of`
    /// carries it through both levels.
    fun mirror_keys(entries: &mut vector<Entry>) {
        each_kv_mut(entries, |k, v| {
            *v = *k;
        });
    }
    spec mirror_keys {
        aborts_if false;
        ensures forall i in 0..len(entries): entries[i].value == old(entries)[i].key;
        ensures forall i in 0..len(entries): entries[i].key == old(entries)[i].key;
    }

    /// The same wrapper expanded twice in one function, over different
    /// targets: each expansion's material — including the expansion-entry
    /// snapshots anchoring the invariants' `old(..)` — is independent.
    fun set_both(a: &mut vector<Entry>, b: &mut vector<Entry>) {
        each_kv_mut(a, |_k, v| {
            *v = 1;
        });
        each_kv_mut(b, |_k, v| {
            *v = 2;
        });
    }
    spec set_both {
        aborts_if false;
        ensures forall i in 0..len(a): a[i].value == 1;
        ensures forall i in 0..len(b): b[i].value == 2;
    }

    /// Non-vacuity canary through the `_mut` chain.
    fun mirror_keys_wrong(entries: &mut vector<Entry>) {
        each_kv_mut(entries, |k, v| {
            *v = *k;
        });
    }
    spec mirror_keys_wrong {
        // error: values mirror the keys
        ensures forall i in 0..len(entries): entries[i].value == old(entries)[i].value;
    }
}
