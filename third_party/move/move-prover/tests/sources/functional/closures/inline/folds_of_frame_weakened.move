// A loop whose `unchanged_of` frame invariants are weakened (the lambda's
// memory footprint cannot be determined exactly) keeps its `folds_of`
// invariant: the frame is not material the fold equation depends on. The
// kept invariant is still checked, so a false fold claim and a lambda that
// writes the table it reads still fail.
module 0x42::folds_of_frame_weakened {
    use std::vector;
    use extensions::table::{Self, Table};

    struct Sheet has store { stores: Table<u64, u64> }

    spec fun spec_fold<T, Acc>(f: |Acc, &T| Acc, v: vector<T>, init: Acc, end: u64): Acc {
        if (end == 0) init
        else result_of<f>(spec_fold(f, v, init, end - 1), v[end - 1])
    }

    /// The generic HOF, with `vector::for_each_ref`'s invariants.
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
            invariant forall x: T: (forall j in 0..i: x != v[j]) ==> unchanged_of<f>(x);
        };
    }

    inline fun table_read(sheet: &Sheet, k: u64): u64 {
        *table::borrow(&sheet.stores, k)
    }
    spec table_read {
        pragma opaque;
        aborts_if !table::spec_contains(sheet.stores, k);
        ensures result == table::spec_get(sheet.stores, k);
    }

    fun table_read_noinline(sheet: &Sheet, k: u64): u64 {
        *table::borrow(&sheet.stores, k)
    }
    spec table_read_noinline {
        pragma opaque;
        aborts_if !table::spec_contains(sheet.stores, k);
        ensures result == table::spec_get(sheet.stores, k);
    }

    fun sum(sheet: &Sheet, keys: &vector<u64>): u64 {
        let total = 0;
        each_ref(keys, |k| { total = total + table_read(sheet, *k); });
        total
    }
    spec sum {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, u64>(
            |acc, k| acc + table::spec_get(sheet.stores, k), keys, 0, len(keys));
    }

    fun sum_noinline(sheet: &Sheet, keys: &vector<u64>): u64 {
        let total = 0;
        each_ref(keys, |k| { total = total + table_read_noinline(sheet, *k); });
        total
    }
    spec sum_noinline {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, u64>(
            |acc, k| acc + table::spec_get(sheet.stores, k), keys, 0, len(keys));
    }

    fun sum_false(sheet: &Sheet, keys: &vector<u64>): u64 {
        let total = 0;
        each_ref(keys, |k| { total = total + table_read(sheet, *k); });
        total
    }
    spec sum_false {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, u64>( // error: one per element too many
            |acc, k| acc + table::spec_get(sheet.stores, k) + 1, keys, 0, len(keys));
    }

    // The lambda overwrites the entry it reads: the fold over the entry
    // table is false when a key repeats.
    fun sum_and_bump(sheet: &mut Sheet, keys: &vector<u64>): u64 {
        let total = 0;
        each_ref(keys, |k| {
            let v = table::borrow_mut(&mut sheet.stores, *k);
            total = total + *v;
            *v = *v + 1;
        });
        total
    }
    spec sum_and_bump {
        pragma aborts_if_is_partial;
        ensures result == spec_fold<u64, u64>( // error: repeated keys read bumped values
            |acc, k| acc + table::spec_get(old(sheet).stores, k), keys, 0, len(keys));
    }
}
