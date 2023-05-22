spec aptos_std::big_vector {
    // -----------------
    // Data invariants
    // -----------------

    spec BigVector {
        invariant bucket_size != 0;
        invariant spec_table_len(buckets) == 0 ==> end_index == 0;
        invariant end_index == 0 ==> spec_table_len(buckets) == 0;
        invariant end_index <= spec_table_len(buckets) * bucket_size;

        // ensure all buckets except last has `bucket_size`
        invariant spec_table_len(buckets) == 0
            || (forall i in 0..spec_table_len(buckets)-1: len(table_with_length::spec_get(buckets, i)) == bucket_size);
        // ensure last bucket doesn't have more than `bucket_size` elements
        invariant spec_table_len(buckets) == 0
            || len(table_with_length::spec_get(buckets, spec_table_len(buckets) -1 )) <= bucket_size;
        // ensure each table entry exists due to a bad spec in `Table::spec_get`
        invariant forall i in 0..spec_table_len(buckets): spec_table_contains(buckets, i);
        // ensure correct number of buckets
        invariant spec_table_len(buckets) == (end_index + bucket_size - 1) / bucket_size;
        // ensure bucket lengths add up to `end_index`
        invariant (spec_table_len(buckets) == 0 && end_index == 0)
            || (spec_table_len(buckets) != 0 && ((spec_table_len(buckets) - 1) * bucket_size) + (len(table_with_length::spec_get(buckets, spec_table_len(buckets) - 1))) == end_index);
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

    spec empty<T: store>(bucket_size: u64): BigVector<T> {
        aborts_if bucket_size == 0;
        ensures length(result) == 0;
        ensures result.bucket_size == bucket_size;
    }

    spec singleton<T: store>(element: T, bucket_size: u64): BigVector<T> {
        ensures length(result) == 1;
        ensures result.bucket_size == bucket_size;
    }

    spec destroy_empty<T>(v: BigVector<T>) {
        aborts_if !is_empty(v);
    }

    spec borrow<T>(v: &BigVector<T>, i: u64): &T {
        aborts_if i >= length(v);
        ensures result == spec_at(v, i);
    }

    spec borrow_mut<T>(v: &mut BigVector<T>, i: u64): &mut T {
        aborts_if i >= length(v);
        ensures result == spec_at(v, i);
    }

    spec push_back<T: store>(v: &mut BigVector<T>, val: T) {
        let num_buckets = spec_table_len(v.buckets);
        aborts_if false;
        ensures length(v) == length(old(v)) + 1;
        ensures v.end_index == old(v.end_index) + 1;
        ensures spec_at(v, v.end_index-1) == val;
        ensures forall i in 0..v.end_index-1: spec_at(v, i) == spec_at(old(v), i);
    }

    spec pop_back<T>(v: &mut BigVector<T>): T {
        aborts_if is_empty(v);
        ensures length(v) == length(old(v)) - 1;
        ensures result == old(spec_at(v, v.end_index-1));
        ensures forall i in 0..v.end_index: spec_at(v, i) == spec_at(old(v), i);
    }

    spec swap_remove<T>(v: &mut BigVector<T>, i: u64): T {
        pragma verify_duration_estimate = 120;
        aborts_if i >= length(v);
        ensures length(v) == length(old(v)) - 1;
        ensures result == spec_at(old(v), i);
    }

    spec swap<T>(v: &mut BigVector<T>, i: u64, j: u64) {
        pragma verify_duration_estimate = 120;
        aborts_if i >= length(v) || j >= length(v);
        ensures length(v) == length(old(v));
        ensures spec_at(v, i) == spec_at(old(v), j);
        ensures spec_at(v, j) == spec_at(old(v), i);
        ensures forall idx in 0..length(v)
            where idx != i && idx != j:
            spec_at(v, idx) == spec_at(old(v), idx);
    }

    spec append<T: store>(lhs: &mut BigVector<T>, other: BigVector<T>) {
        pragma verify=false;
        pragma unroll = 1;
        //aborts_if false; // TODO: timeout even with unroll=1
        //ensures forall i in 0..length(old(lhs)): spec_at(lhs, i)==spec_at(old(lhs), i); // TODO: timeout even with unroll=1
        //ensures forall i in 0..length(other): spec_at(lhs, length(old(lhs))+i)==spec_at(other, i); // TODO: timeout even with unroll=1
    }

    spec remove<T>(v: &mut BigVector<T>, i: u64): T {
        pragma unroll = 1;
        pragma timeout = 120;
        aborts_if i >= length(v);
        ensures length(v) == length(old(v)) - 1;
        //ensures forall j in 0..i: spec_at(v, j)==spec_at(old(v), j); // TODO: taking long time
        //ensures forall j in i..length(v)-1: spec_at(v, j)==spec_at(old(v), j+1); // TODO: taking long time
        ensures old(spec_at(v, i)) == result;
    }

    spec reverse<T>(v: &mut BigVector<T>) {
        // pragma verify=false;
        pragma unroll = 1;
        // TODO: data invariant doesn't hold.
        aborts_if false;
        //ensures forall i in 0..length(v): spec_at(v, i) == spec_at(old(v), length(v)-1-i);
    }

    spec index_of<T>(v: &BigVector<T>, val: &T): (bool, u64) {
        pragma unroll = 3;
        aborts_if false;
        //ensures result_1 == (exists i in 0..length(v): spec_at(v, i)==val); // whether v contains val or not // TODO: false negative. lack of trigger?
        ensures result_1 ==> spec_at(v, result_2) == val; // if true, return the index where v contains val
        ensures result_1 ==> (forall i in 0..result_2: spec_at(v, i)!=val); // ensure the smallest index
        ensures !result_1 ==> result_2 == 0; // return 0 if v does not contain val
    }


    spec contains<T>(v: &BigVector<T>, val: &T): bool {
        pragma unroll = 3;
        aborts_if false;
        // ensures result == (exists i in 0..length(v): spec_at(v, i)==val); // TODO: false negative. lack of trigger?
    }

    spec length<T>(v: &BigVector<T>): u64 {
        ensures result == v.end_index;
    }

    spec is_empty<T>(v: &BigVector<T>): bool {
        ensures result == (length(v) == 0);
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

    spec fun spec_at<T>(v: BigVector<T>, i: u64): T {
        let bucket = i / v.bucket_size;
        let idx = i % v.bucket_size;
        let v = table_with_length::spec_get(v.buckets, bucket);
        v[idx]
    }
}
