module 0x42::test {

    struct X<M> has copy, drop, store {
        value: M
    }
    struct Y<T> has key, drop {
        field: T
    }

    fun test_resource_4() acquires Y {
        let addr = @0x1;
        let y = borrow_global_mut<Y<X<bool>>>(addr);
        y.field.value = false;
        spec {
            assert Y<X<bool>>[addr].field.value == false;
        };
    }

}
