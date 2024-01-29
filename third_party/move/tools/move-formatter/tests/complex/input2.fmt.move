module TestFunFormat {
    /* test two fun Close together without any blank lines, and here is a BlockComment */
    public fun multi_arg33(p1: u64, p2: u64): u64 {
        p1 + p2
    }

    spec singletonlllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllllll<T: store>(
        element: T, bucket_size: u64
    ): BigVector<T>{
        ensures length(result) == 1;
        ensures result.bucket_size == bucket_size;
    }
}