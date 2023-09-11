spec aptos_std::big_vector_model {
    // -----------------
    // Data invariants
    // -----------------

    spec BigVectorModel {
        invariant spec_table_len(buckets) == 0 ==> end_index == 0;
        invariant end_index == 0 ==> spec_table_len(buckets) == 0;
        invariant end_index <= spec_table_len(buckets) * BUCKET_SIZE;

        // ensure all buckets except last has `bucket_size`
        invariant spec_table_len(buckets) == 0
            || (forall i in 0..spec_table_len(buckets)-1: len(table_with_length::spec_get(buckets, i)) == BUCKET_SIZE);
        // ensure last bucket doesn't have more than `bucket_size` elements
        invariant spec_table_len(buckets) == 0
            || len(table_with_length::spec_get(buckets, spec_table_len(buckets) -1 )) <= BUCKET_SIZE;
        // ensure each table entry exists due to a bad spec in `Table::spec_get`
        invariant forall i in 0..spec_table_len(buckets): spec_table_contains(buckets, i);
        // ensure correct number of buckets
        invariant spec_table_len(buckets) == (end_index + BUCKET_SIZE - 1) / BUCKET_SIZE;
        // ensure bucket lengths add up to `end_index`
        invariant (spec_table_len(buckets) == 0 && end_index == 0)
            || (spec_table_len(buckets) != 0 && ((spec_table_len(buckets) - 1) * BUCKET_SIZE) + (len(table_with_length::spec_get(buckets, spec_table_len(buckets) - 1))) == end_index);
        // ensures that no out-of-bound buckets exist
        invariant forall i: u64 where i >= spec_table_len(buckets):  {
            !spec_table_contains(buckets, i)
        };
        // ensures that all buckets exist
        invariant forall i: u64 where i < spec_table_len(buckets):  {
            spec_table_contains(buckets, i)
        };
        // ensures that the last bucket is non-empty
        invariant spec_table_len(buckets) == 0
            || (len(table_with_length::spec_get(buckets, spec_table_len(buckets) - 1)) > 0);
    }


    // -----------------------
    // Function specifications
    // -----------------------

    spec empty<T: store>(): BigVectorModel<T> {
        ensures length(result) == 0;
    }

    spec singleton<T: store>(element: T): BigVectorModel<T> {
        ensures length(result) == 1;
    }

    spec destroy_empty<T>(v: BigVectorModel<T>) {
        aborts_if !is_empty(v);
    }

    spec destroy<T: drop>(v: BigVectorModel<T>) {
        pragma verify=false;
    }

    spec borrow<T>(v: &BigVectorModel<T>, i: u64): &T {
        aborts_if i >= length(v);
        ensures result == spec_at(v, i);
    }

    spec borrow_mut<T>(v: &mut BigVectorModel<T>, i: u64): &mut T {
        aborts_if i >= length(v);
        ensures result == spec_at(v, i);
    }

    spec append<T: store>(lhs: &mut BigVectorModel<T>, other: BigVectorModel<T>) {
        pragma verify=false;
        // ensures forall i in 0..old(lhs).end_index: spec_at(old(lhs), i) == spec_at(lhs, i);
        // ensures forall i in 0..other.end_index: spec_at(other, i) == spec_at(lhs, i + old(lhs).end_index);
    }

    spec push_back<T: store>(v: &mut BigVectorModel<T>, val: T) {
        let num_buckets = spec_table_len(v.buckets);
        aborts_if num_buckets * BUCKET_SIZE > MAX_U64;
        aborts_if v.end_index + 1 > MAX_U64;
        ensures length(v) == length(old(v)) + 1;
        ensures v.end_index == old(v.end_index) + 1;
        ensures spec_at(v, v.end_index-1) == val;
        ensures forall i in 0..v.end_index-1: spec_at(v, i) == spec_at(old(v), i);
    }

    spec pop_back<T>(v: &mut BigVectorModel<T>): T {
        aborts_if is_empty(v);
        ensures length(v) == length(old(v)) - 1;
        ensures result == old(spec_at(v, v.end_index-1));
        ensures forall i in 0..v.end_index: spec_at(v, i) == spec_at(old(v), i);
    }

    spec remove<T>(v: &mut BigVectorModel<T>, i: u64): T {
        aborts_if i >= length(v);
        ensures result == spec_at(old(v), i);
        // ensures forall j in 0..(i): spec_at(v, j) == spec_at(old(v), j);
        // ensures forall j in (i+1)..old(v).end_index: spec_at(v, j - 1) == spec_at(old(v), j);
    }

    spec swap_remove<T>(v: &mut BigVectorModel<T>, i: u64): T {
        pragma verify_duration_estimate = 120;
        aborts_if i >= length(v);
        ensures length(v) == length(old(v)) - 1;
        ensures result == spec_at(old(v), i);
    }

    spec swap<T>(v: &mut BigVectorModel<T>, i: u64, j: u64) {
        pragma verify_duration_estimate = 120;
        aborts_if i >= length(v) || j >= length(v);
        ensures length(v) == length(old(v));
        ensures spec_at(v, i) == spec_at(old(v), j);
        ensures spec_at(v, j) == spec_at(old(v), i);
        ensures forall idx in 0..length(v)
            where idx != i && idx != j:
            spec_at(v, idx) == spec_at(old(v), idx);
    }

    spec reverse<T>(v: &mut BigVectorModel<T>) {
        pragma verify=false;
        // ensures forall i in 0..v.end_index: spec_at(v, i) == spec_at(old(v), v.end_index - (i + 1));
    }

    spec index_of<T>(v: &BigVectorModel<T>, val: &T): (bool, u64) {
        ensures (result_1 == true) ==> (spec_at(v, result_2) == val);
        // ensures (result_1 == false) ==> ((forall i in 0..v.end_index: spec_at(v, i) != val) && (result_2 == 0));
    }

    // ---------------------
    // Spec helper functions
    // ---------------------

    spec fun spec_table_len<K, V>(t: TableWithLength<K, V>): u64 {
        table_with_length::spec_len(t)
    }

    spec fun spec_table_contains<K, V>(t: TableWithLength<K, V>, k: K): bool {
        table_with_length::spec_contains(t, k)
    }

    spec fun spec_at<T>(v: BigVectorModel<T>, i: u64): T {
        let bucket = i / BUCKET_SIZE;
        let idx = i % BUCKET_SIZE;
        let v = table_with_length::spec_get(v.buckets, bucket);
        v[idx]
    }
}
