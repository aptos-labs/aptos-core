spec aptos_std::big_vector {
    spec BigVector {
        invariant bucket_size > 0;

        invariant table_with_length::spec_len(buckets) == 0 ==> end_index == 0;

        invariant end_index == 0 ==> table_with_length::spec_len(buckets) == 0;

        //invariant num_buckets == table_with_length::spec_len(buckets);

        invariant {
            let num_buckets = table_with_length::spec_len(buckets);
            num_buckets > 0 ==>
                end_index == (num_buckets-1) * bucket_size
                    + len(table_with_length::spec_get(buckets, num_buckets-1))
        };

        invariant {
            let num_buckets = table_with_length::spec_len(buckets);
            num_buckets > 0 ==>
                (forall i in 0..num_buckets-1: (
                    table_with_length::spec_contains(buckets, i) &&
                    len(table_with_length::spec_get(buckets, i)) == bucket_size
                ))
        };

        invariant {
            let num_buckets = table_with_length::spec_len(buckets);
            num_buckets > 0 ==> (
                table_with_length::spec_contains(buckets, num_buckets - 1) &&
                    len(table_with_length::spec_get(buckets, num_buckets - 1)) > 0
            )
        };

        invariant {
            let num_buckets = table_with_length::spec_len(buckets);
            (end_index == num_buckets * bucket_size) ==> (
                forall i in 0..num_buckets: (
                    table_with_length::spec_contains(buckets, i) &&
                        len(table_with_length::spec_get(buckets, i)) == bucket_size
                )
            )
        };

        invariant forall i: u64 where i >= table_with_length::spec_len(buckets):  {
            !table_with_length::spec_contains(buckets, i)
        };

        invariant forall i: u64 where i < table_with_length::spec_len(buckets):  {
            table_with_length::spec_contains(buckets, i)
        };

        invariant end_index <= table_with_length::spec_len(buckets) * bucket_size;
    }

    spec swap<T>(v: &mut BigVector<T>, i: u64, j: u64) {
        pragma verify=false;
        pragma opaque;
        // It takes 57.178s to prove this spec.
        aborts_if i >= length(v) || j >= length(v);
        ensures length(v) == length(old(v));
        ensures spec_at(v, i) == spec_at(old(v), j);
        ensures spec_at(v, j) == spec_at(old(v), i);
        ensures forall idx in 0..length(v)
            where idx != i && idx != j:
                spec_at(v, idx) == spec_at(old(v), idx);

        // alternative definition
        // ensures forall idx in 0..length(v):
        //     (idx == i || idx == j || spec_at(v, idx) == spec_at(old(v), idx));

        // ensures spec_at_abs(v, i) == spec_at_abs(old(v), j);
        // ensures spec_at_abs(v, j) == spec_at_abs(old(v), i);
        // ensures forall idx in 0..length(v)
        //     where idx != i && idx != j:
        //     spec_at_abs(v, idx) == spec_at_abs(old(v), idx);

        ensures v.end_index == old(v).end_index;
        ensures v.bucket_size == old(v).bucket_size;
        //ensures v.num_buckets == old(v).num_buckets;
    }

    spec swap_remove<T>(v: &mut BigVector<T>, i: u64): T {
        aborts_if i >= length(v);
        ensures length(v) == length(old(v)) - 1;
        ensures result == spec_at(old(v), i);
    }

    spec empty<T: store>(bucket_size: u64): BigVector<T> {
        aborts_if bucket_size == 0;
        ensures is_empty(result);
    }

    spec singleton<T: store>(element: T, bucket_size: u64): BigVector<T> {
        aborts_if bucket_size == 0;
        ensures length(result) == 1;
        ensures spec_at(result, 0) == element;
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
        aborts_if false;
        ensures v.end_index == old(v.end_index) + 1;
        ensures spec_at(v, v.end_index-1) == val;
        // disabled to reduce the running time.
        //ensures forall i in 0..v.end_index-1: spec_at(v, i) == spec_at(old(v), i); // takes 29.947s.
    }

    spec pop_back<T>(v: &mut BigVector<T>): T {
        aborts_if is_empty(v);
        ensures v.end_index == old(v.end_index) - 1;
        ensures result == old(spec_at(v, v.end_index-1));
        ensures forall i in 0..v.end_index: spec_at(v, i) == spec_at(old(v), i);
    }

    spec destroy_empty<T>(v: BigVector<T>) {
        aborts_if !is_empty(v);
    }

    spec fun spec_at_abs<T>(v: BigVector<T>, i: u64): T;

    spec fun spec_at<T>(v: BigVector<T>, i: u64): T {
        let bucket = i / v.bucket_size;
        let idx = i % v.bucket_size;
        let v = table_with_length::spec_get(v.buckets, bucket);
        v[idx]
    }

    spec append<T: store>(lhs: &mut BigVector<T>, other: BigVector<T>) {
        pragma verify=false;
    }

    spec remove<T>(v: &mut BigVector<T>, i: u64): T {
        //pragma verify=false;
        //aborts_if i >= length(v);
        pragma verify=false;
    }

    spec reverse<T>(v: &mut BigVector<T>) {
        pragma verify=false;
    }

    spec index_of<T>(v: &BigVector<T>, val: &T): (bool, u64) {
        pragma verify=false;
    }
}
