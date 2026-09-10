// A supplied invariant can yield a large quantified contract behind an
// entry-state path guard without making the invariant incomplete.
// flag: --generate-only
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
    spec partition(values: &mut vector<u64>, pivot: u64): u64 {
        pragma opaque = true, aborts_if_is_partial = true;
        let cse_ = len(values) >= 1 && values[len(values) - 1] == update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1];
        ensures [inferred = sathard] len(values) == len(old(values)) && (len(old(values)) == len(values) && (len(old(values)) >= 1 && values[len(old(values)) - 1] == update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1])) ==> (forall y: u64, z: vector<u64>: len(z) == len(old(values)) && len(old(values)) == len(z) && y <= len(old(values)) - 1 && z[len(old(values)) - 1] == update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1] && (forall x: num: (0 <= x && x < y ==> z[x] < update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1]) && (y <= x && x < len(old(values)) - 1 ==> z[x] >= update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1])) ==> result == y);
        ensures [inferred = sathard] len(values) == len(old(values)) && (len(old(values)) == len(values) && (len(old(values)) >= 1 && values[len(old(values)) - 1] == update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1])) ==> (forall y: u64, z: vector<u64>: len(z) == len(old(values)) && len(old(values)) == len(z) && y <= len(old(values)) - 1 && z[len(old(values)) - 1] == update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1] && (forall x: num: (0 <= x && x < y ==> z[x] < update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1]) && (y <= x && x < len(old(values)) - 1 ==> z[x] >= update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1])) ==> values == update(update(z, y, z[len(old(values)) - 1]), len(old(values)) - 1, z[y]));
        ensures [inferred = sathard] len(values) == len(old(values)) && (len(old(values)) == len(values) && (len(old(values)) >= 1 && values[len(old(values)) - 1] == update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1])) ==> (forall y: u64, z: u64, x1: vector<u64>: len(x1) == len(old(values)) && (len(old(values)) == len(x1) && (y <= z && z <= len(old(values)) - 1 && (x1[len(old(values)) - 1] == update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1] && ((forall x: num: (0 <= x && x < y ==> x1[x] < update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1]) && (y <= x && x < z ==> x1[x] >= update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1])) && (z < len(old(values)) - 1 && update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[z] >= update(update(old(values), pivot, old(values)[len(old(values)) - 1]), len(old(values)) - 1, old(values)[pivot])[len(old(values)) - 1]))))) ==> values == x1);
        aborts_if [inferred] len(values) == 0;
        aborts_if [inferred] !in_range(values, pivot) || !in_range(values, len(values) - 1);
        aborts_if [inferred] !in_range(update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot]), len(values) - 1);
        aborts_if [inferred = sathard] cse_ && (exists y: u64, z: vector<u64>: y <= len(values) - 1 && len(z) == len(values) && len(values) == len(z) && z[len(values) - 1] == update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1] && (forall x: num: (0 <= x && x < y ==> z[x] < update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1]) && (y <= x && x < len(values) - 1 ==> z[x] >= update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1])) && (!in_range(z, y) || !in_range(z, len(values) - 1)));
        aborts_if [inferred = sathard] cse_ && (exists y: u64, z: u64, x1: vector<u64>: y <= z && z <= len(values) - 1 && z < len(values) - 1 && len(x1) == len(values) && len(values) == len(x1) && x1[len(values) - 1] == update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1] && (forall x: num: (0 <= x && x < y ==> x1[x] < update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1]) && (y <= x && x < z ==> x1[x] >= update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1])) && !in_range(x1, z));
        aborts_if [inferred = sathard] cse_ && (exists y: u64, z: u64, x1: vector<u64>: y <= z && z <= len(values) - 1 && z < len(values) - 1 && len(x1) == len(values) && len(values) == len(x1) && x1[len(values) - 1] == update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1] && (forall x: num: (0 <= x && x < y ==> x1[x] < update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1]) && (y <= x && x < z ==> x1[x] >= update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1])) && x1[z] < update(update(values, pivot, values[len(values) - 1]), len(values) - 1, values[pivot])[len(values) - 1] && (!in_range(x1, z) || !in_range(x1, y)));
    }

}
/*
Inference diagnostics:
warning: WP could not characterize the aborts of `guarded_loop_summary::partition` exactly, so its emitted `aborts_if` clauses are a lower bound and the specification carries `aborts_if_is_partial`. Complete the abort behavior and remove that pragma before relying on the contract. Reasons:
  = an emitted abort condition is flagged `vacuous` or `sathard`
   ┌─ tests/inference/guarded_loop_summary.move:6:5
   │
 6 │ ╭     fun partition(values: &mut vector<u64>, pivot: u64): u64 {
 7 │ │         let last = values.length() - 1;
 8 │ │         values.swap(pivot, last);
 9 │ │         let p = values[last];
   · │
32 │ │         store
33 │ │     }
   │ ╰─────^

Verification: Succeeded.
*/
