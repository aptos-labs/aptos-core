// Transitivity of `folds_of` through pure forwarding wrappers (the
// anchored deferral): a wrapper HOF passes its function-typed parameter on
// to an inner inline HOF inside a forwarding lambda. The inner HOF's
// `folds_of` invariant over the forwarder is rewritten to an *anchored*
// general-form occurrence over the wrapper's parameter — the iteration
// arguments composed through the forwarder — and resolves against the
// concrete lambda when the wrapper is expanded at its own call sites. The
// `FoldsCaptureAnchor` marker placed at the inner expansion's entry
// records where the concrete lambda's captures are snapshotted, so the
// fold equation's base values survive the deferral (per entry into the
// inner expansion). This mirrors `smart_table::for_each_ref` over its
// `borrow_kv` projection prelude.
module 0x42::folds_of_wrapper {
    use std::vector;

    /// The inner HOF with the `folds_of` invariant (element form).
    inline fun each_ref<T>(v: &vector<T>, f: |&T|) {
        let i = 0;
        let n = vector::length(v);
        while (i < n) {
            f(vector::borrow(v, i));
            i = i + 1;
        } spec {
            invariant i <= n;
            invariant n == len(v);
            invariant folds_of<f>(v, i);
        };
    }

    /// The index-fold recursion specialized by the deferred (general-form)
    /// occurrences, resolved in the current module.
    spec fun spec_fold_idx<Acc>(t: |Acc, u64| Acc, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<t>(spec_fold_idx(t, init, end - 1), end - 1)
    }

    // ===== The two-level `smart_table` mirror =====

    struct Entry has copy, drop, store {
        key: u64,
        value: u64,
    }

    fun borrow_kv(self: &Entry): (&u64, &u64) {
        (&self.key, &self.value)
    }

    /// Forwarding wrapper with the `borrow_kv` projection prelude.
    inline fun each_kv_ref(entries: &vector<Entry>, f: |&u64, &u64|) {
        each_ref(entries, |elem| {
            let (key, value) = elem.borrow_kv();
            f(key, value)
        });
    }

    /// A capture-writing accumulation through the two-level chain: the
    /// deferred occurrence resolves here, folding the concrete lambda's
    /// transformer over the composed iteration arguments
    /// `(entries[j].key, entries[j].value)`.
    fun sum_values(entries: &vector<Entry>): u64 {
        let sum = 0;
        each_kv_ref(entries, |_k, v| sum = sum + *v);
        sum
    }
    spec sum_values {
        pragma aborts_if_is_partial;
        ensures result == spec_fold_idx<u64>(|acc, j| acc + entries[j].value, 0, len(entries));
    }

    /// Both composed arguments in use.
    fun sum_kv(entries: &vector<Entry>): u64 {
        let sum = 0;
        each_kv_ref(entries, |k, v| sum = sum + *k + *v);
        sum
    }
    spec sum_kv {
        pragma aborts_if_is_partial;
        ensures result
            == spec_fold_idx<u64>(
                |acc, j| acc + entries[j].key + entries[j].value, 0, len(entries)
            );
    }

    /// The collection pattern through the chain, restated directly against
    /// the composed fold (a bridging lemma identifying the fold with the
    /// projected prefix would strengthen this to element-wise properties,
    /// as in `folds_of_collect.move` for the single-level pattern).
    fun collect_keys(entries: &vector<Entry>): vector<u64> {
        let keys = vector[];
        each_kv_ref(entries, |k, _v| vector::push_back(&mut keys, *k));
        keys
    }
    spec collect_keys {
        aborts_if false;
        ensures result
            == spec_fold_idx<vector<u64>>(
                |acc, j| concat(acc, vec(entries[j].key)), vec(), len(entries)
            );
    }

    // ===== The three-level chain =====

    /// A second forwarding level: the anchored occurrence re-defers
    /// through the identity forwarder, keeping its anchor.
    inline fun each_kv_fwd(entries: &vector<Entry>, f: |&u64, &u64|) {
        each_kv_ref(entries, |k, v| f(k, v));
    }

    fun sum_values_three_levels(entries: &vector<Entry>): u64 {
        let sum = 0;
        each_kv_fwd(entries, |_k, v| sum = sum + *v);
        sum
    }
    spec sum_values_three_levels {
        pragma aborts_if_is_partial;
        ensures result == spec_fold_idx<u64>(|acc, j| acc + entries[j].value, 0, len(entries));
    }

    /// The same wrapper expanded twice in one function: the anchor labels
    /// of the two expansions' deferred occurrences (and their markers) are
    /// freshened per expansion, so the resolutions stay independent.
    fun sum_values_both(a: &vector<Entry>, b: &vector<Entry>): u64 {
        let sum_a = 0;
        let sum_b = 0;
        each_kv_ref(a, |_k, v| sum_a = sum_a + *v);
        each_kv_ref(b, |_k, v| sum_b = sum_b + *v);
        sum_a + sum_b
    }
    spec sum_values_both {
        pragma aborts_if_is_partial;
        ensures result
            == spec_fold_idx<u64>(|acc, j| acc + a[j].value, 0, len(a))
                + spec_fold_idx<u64>(|acc, j| acc + b[j].value, 0, len(b));
    }

    /// The intermediate inline function has no specification. Its fold
    /// summary must survive until the specified outer caller is expanded.
    inline fun sum_values_inline(entries: &vector<Entry>): u64 {
        let sum = 0;
        each_kv_ref(entries, |_k, v| sum = sum + *v);
        sum
    }

    fun sum_values_through_inline(entries: &vector<Entry>): u64 {
        sum_values_inline(entries)
    }
    spec sum_values_through_inline {
        pragma aborts_if_is_partial;
        ensures result == spec_fold_idx<u64>(|acc, j| acc + entries[j].value, 0, len(entries));
    }

    // ===== Non-vacuity canary =====

    fun sum_values_wrong(entries: &vector<Entry>): u64 {
        let sum = 0;
        each_kv_ref(entries, |_k, v| sum = sum + *v);
        sum
    }
    spec sum_values_wrong {
        pragma aborts_if_is_partial;
        // error: off by one
        ensures result
            == spec_fold_idx<u64>(|acc, j| acc + entries[j].value, 0, len(entries)) + 1;
    }
}
