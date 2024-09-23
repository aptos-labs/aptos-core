module 0x42::test {

    struct X<M> has copy, drop, store {
        value: M
    }

    fun test_struct_no_resource() acquires X {
        let addr = @0x1;
        let _ = X<bool>[addr];
    }

}
