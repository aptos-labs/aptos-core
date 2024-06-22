module 0x42::test {

    struct X<M> has copy, drop, store {
        value: M
    }
    struct Y<T> has key, drop {
        field: T
    }

    fun test_resource_no_copy() acquires Y {
        let addr = @0x1;
        let _ = Y<X<bool>>[addr];
    }

}
