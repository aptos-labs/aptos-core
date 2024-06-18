module 0x42::test {

    struct R has key, drop { value: bool }

    spec schema Test {
    }

    fun test_no_ref_for_resource() acquires R {
        use 0x42::test;
        assert!((test::R[@0x1]).value == true, 0);
    }

    fun test_no_schema() {
        Test[@0x1];
    }

    fun test_no_schema_ref() {
        use 0x42::test;
        &test::Test[@0x1];
    }

}
