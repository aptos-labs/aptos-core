/// test_point: fun name too long
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

 spec empty<T: store>(bucket_size: u64): BigVector<T> {
    aborts_if bucket_size == 0;
    ensures length(result) == 0;
    ensures result.bucket_size == bucket_size;
}

spec singletonlllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllll<T: store>(element: T, bucket_size: u64): BigVector<T> {
    ensures length(result) == 1;
    ensures result.bucket_size == bucket_size;
}
}
