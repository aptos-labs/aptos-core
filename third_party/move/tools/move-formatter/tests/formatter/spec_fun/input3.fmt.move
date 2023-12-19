/// test_point: comment between two functions
spec aptos_std::big_vector {
    // -----------------
    // Data invariants
    // -----------------

    spec BigVector {
        invariant bucket_size != 0;
        invariant spec_table_len(buckets) == 0 ==> end_index == 0;
        invariant end_index == 0 ==> spec_table_len(buckets) == 0;
        invariant end_index <= spec_table_len(buckets) * bucket_size;
    }

    spec fun spec_table_len<K, V>(t: TableWithLength<K, V>): u64 {
        table_with_length::spec_len(t)
    }

    // comment
    spec fun spec_table_contains<K, V>(t: TableWithLength<K, V>, k: K): bool {
        table_with_length::spec_contains(t, k)
    }

    /*comment*/ 
    spec fun spec_at<T>(v: BigVector<T>, i: u64): T {
        let bucket = i / v.bucket_size;
        let idx = i % v.bucket_size;
        let v = table_with_length::spec_get(v.buckets, bucket);
        v[idx]
    }
}