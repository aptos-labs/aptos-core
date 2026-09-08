// A supplied invariant must not hide residual quantified WP obligations
// behind an entry-state path guard (historical QP-part-025 r03).
// reject-incomplete-inference: true
module 0x42::guarded_loop_summary {

    fun partition(values: &mut vector<u64>, pivot: u64): u64 {
        let last = values.length() - 1;
        values.swap(pivot, last);
        let p = values[last];
        let store = 0;
        let i = 0;
        while (i < last) {
            if (values[i] < p) {
                values.swap(i, store);
                store += 1;
            };
            i += 1;
        } spec {
            // [inferred] The sweep keeps the vector's shape, so `last` stays the
            // final index and every access below is in range.
            invariant [inferred] len(values) == len(old(values));
            invariant [inferred] last == len(values) - 1;
            invariant [inferred] store <= i && i <= last;
            // [inferred] The pivot is parked at `last`; the loop never touches it.
            invariant [inferred] values[last] == p;
            // [inferred] `[0, store)` is the growing "below pivot" prefix and
            // `[store, i)` is the scanned remainder, all at or above the pivot.
            invariant [inferred] forall j: num: 0 <= j && j < store ==> values[j] < p;
            invariant [inferred] forall j: num: store <= j && j < i ==> values[j] >= p;
        };
        values.swap(store, last);
        store
    }
}
