spec aptos_std::big_vector {
    spec BigVector {
        invariant bucket_size != 0;
        // ensure all buckets except last has `bucket_size`
        invariant table_with_length::spec_len(buckets) == 0 || (forall i in 0..table_with_length::spec_len(buckets)-1: vector::length(table_with_length::spec_get(buckets, i)) == bucket_size);
        // ensure last bucket doesn't have more than `bucket_size` elements
        invariant table_with_length::spec_len(buckets) == 0 || vector::length(table_with_length::spec_get(buckets, table_with_length::spec_len(buckets) -1 )) <= bucket_size;
        // ensure each table entry exists due to a bad spec in `Table::spec_get`
        invariant (forall i in 0..table_with_length::spec_len(buckets): table_with_length::spec_contains(buckets, i));
        // ensure correct number of buckets
        invariant table_with_length::spec_len(buckets) == (end_index + bucket_size - 1) / bucket_size;
        // ensure bucket lengths add up to `end_index`
        invariant (table_with_length::spec_len(buckets) == 0 && end_index == 0)
            || (table_with_length::spec_len(buckets) != 0 && ((table_with_length::spec_len(buckets) - 1) * bucket_size) + (vector::length(table_with_length::spec_get(buckets, table_with_length::spec_len(buckets) - 1))) == end_index);
    }

    spec empty {
        aborts_if bucket_size == 0;
        ensures(length(result) == 0);
        ensures(result.bucket_size == bucket_size);
    }

    spec singleton {
        ensures(length(result) == 1);
        ensures(result.bucket_size == bucket_size);
    }

    spec destroy_empty {
        aborts_if !is_empty(v);
    }

    spec borrow {
        aborts_if i >= length(v);
        ensures result == vector::borrow(table_with_length::spec_get(v.buckets, i/v.bucket_size), i % v.bucket_size);
    }

    spec borrow_mut {
        aborts_if i >= length(v);
        ensures result == vector::borrow(table_with_length::spec_get(v.buckets, i/v.bucket_size), i % v.bucket_size);
    }

    spec push_back {
        ensures(length(v) == length(old(v)) + 1);
        ensures val == vector::borrow(table_with_length::spec_get(v.buckets, (length(v)-1)/v.bucket_size), (length(v)-1) % v.bucket_size);
    }

    spec pop_back {
        aborts_if is_empty(v);
        ensures(length(v) == length(old(v)) - 1);
        ensures result == vector::borrow(table_with_length::spec_get(old(v).buckets, (length(old(v))-1)/v.bucket_size), (length(old(v))-1) % v.bucket_size);
    }

    spec swap_remove {
        aborts_if i >= length(v);
        ensures(length(v) == length(old(v)) - 1);
    }

    spec swap {
        aborts_if i >= length(v) || j >= length(v);
        ensures(length(v) == length(old(v)));
    }
}
