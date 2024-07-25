module 0x42::test {

    struct X<M> has copy, drop, store {
        value: M
    }
    struct Y<T> has key, drop {
        field: T
    }

}

module 0x42::test2 {

    fun test_resource_other_module() acquires 0x42::test::Y {
        let addr = @0x1;
        assert!((&0x42::test::Y<0x42::test::X<bool>>[addr]).field.value == true, 1);
        spec {
            // This is OK
            assert 0x42::test::Y<0x42::test::X<bool>>[addr].field.value == true;
        }
    }

}
