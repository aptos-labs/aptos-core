// Transitivity of behavioral predicates through pure forwarding wrappers.
//
// A wrapper HOF passes its function-typed parameter on to an inner inline
// HOF inside a forwarding lambda (an effect-free prelude — here reference
// projections through a pure helper — followed by the application of the
// wrapper's parameter). When the inner HOF is expanded into the wrapper,
// the behavioral predicates of its loop invariants over the forwarding
// lambda resolve to *deferred* predicates over the wrapper's parameter:
// pointwise `ensures_of`/`aborts_of`/`result_of` flow through the derived
// composition, and `unchanged_of` delegates the application's footprint.
// When the wrapper is expanded at its own call sites, the deferred
// predicates resolve once more against the concrete lambda supplied there.
// This mirrors `smart_table::for_each_ref/for_each_mut` (the `borrow_kv`
// projection prelude) and the identity forwarder of
// `sigma_protocol_representation_vec::for_each_ref`.
module 0x42::bp_forwarding {
    use std::vector;

    // ===== Inner inline HOFs with the vector-module invariant sets =====

    inline fun each_ref<T>(v: &vector<T>, f: |&T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant forall j in 0..i: ensures_of<f>(v[j]);
            invariant folds_of<f>(v, i);
            invariant forall j in i..n: unchanged_of<f>(v[j]);
        };
    }

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

    inline fun all_of<T>(v: &vector<T>, p: |&T| bool): bool {
        let result = true;
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            result = p(vector::borrow(v, i));
            if (!result) break;
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant result;
            invariant forall j in 0..i: result_of<p>(v[j]);
        };
        spec {
            assert result <==> (forall j in 0..len(v): result_of<p>(v[j]));
        };
        result
    }

    /// The fold recursion specialized by `folds_of` (restatable by
    /// callers); resolved in the current module.
    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    // ===== The `smart_table` projection shape =====

    struct Entry has copy, drop, store {
        key: u64,
        value: u64,
    }

    /// Reference-projection helper, as in `smart_table::borrow_kv`.
    fun borrow_kv(self: &Entry): (&u64, &u64) {
        (&self.key, &self.value)
    }

    fun borrow_kv_mut(self: &mut Entry): (&mut u64, &mut u64) {
        (&mut self.key, &mut self.value)
    }

    /// Forwarding wrapper with a projection prelude: the returned
    /// references are places of the forwarding lambda's derivation, and
    /// the application of `f` defers to this wrapper's own expansion.
    inline fun each_kv_ref(entries: &vector<Entry>, f: |&u64, &u64|) {
        each_ref(entries, |elem| {
            let (key, value) = elem.borrow_kv();
            f(key, value)
        });
    }

    inline fun each_kv_mut(entries: &mut vector<Entry>, f: |&u64, &mut u64|) {
        each_mut(entries, |elem| {
            let (key, value) = elem.borrow_kv_mut();
            f(key, value)
        });
    }

    inline fun all_kv(entries: &vector<Entry>, p: |&u64, &u64| bool): bool {
        all_of(entries, |elem| {
            let (key, value) = elem.borrow_kv();
            p(key, value)
        })
    }

    // ===== Callers: the deferred predicates resolve here =====

    /// Writes through the projected `&mut` reference; the dual-form
    /// `ensures_of` and the no-abort invariant of `each_mut` constrain the
    /// concrete lambda through two expansion levels.
    fun set_values(entries: &mut vector<Entry>) {
        each_kv_mut(entries, |_k, v| {
            *v = 1;
        });
    }
    spec set_values {
        aborts_if false;
        ensures forall i in 0..len(entries): entries[i].value == 1;
        ensures forall i in 0..len(entries): entries[i].key == old(entries)[i].key;
    }

    /// Aborts compose through the projection prelude: the concrete
    /// lambda's assert appears in the deferred prefix no-abort condition,
    /// over the projected key of each processed element.
    fun check_keys_bounded(entries: &vector<Entry>) {
        each_kv_ref(entries, |k, _v| assert!(*k <= 1000, 2));
    }
    spec check_keys_bounded {
        aborts_if exists i in 0..len(entries): entries[i].key > 1000;
    }

    /// Result forwarding: `result_of` composes through the projection
    /// prelude and resolves against the concrete predicate lambda.
    fun all_values_bounded(entries: &vector<Entry>): bool {
        all_kv(entries, |_k, v| *v <= 100)
    }
    spec all_values_bounded {
        aborts_if false;
        ensures result <==> (forall i in 0..len(entries): entries[i].value <= 100);
    }

    // ===== The sigma identity-forwarder shape =====

    /// Identity forwarder, as in
    /// `sigma_protocol_representation_vec::for_each_ref`.
    inline fun each_fwd<T>(v: &vector<T>, lambda: |&T|) {
        each_ref(v, |x| lambda(x))
    }

    fun check_all_positive(v: &vector<u64>) {
        each_fwd(v, |x| assert!(*x > 0, 1));
    }
    spec check_all_positive {
        aborts_if exists i in 0..len(v): v[i] == 0;
    }

    // ===== Canary: verification is engaged through both levels =====

    fun set_values_wrong(entries: &mut vector<Entry>) {
        each_kv_mut(entries, |_k, v| {
            *v = 1;
        });
    }
    spec set_values_wrong {
        ensures forall i in 0..len(entries): entries[i].value == 2; // error: values are 1
    }
}
