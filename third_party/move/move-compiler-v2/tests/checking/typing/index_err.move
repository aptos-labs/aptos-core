module 0x42::test {

    struct R has key, drop { value: bool }

    spec schema UNUSED_Test {
    }

    fun test_no_ref_for_resource() acquires R {
        use 0x42::test;
        assert!((test::R[@0x1]).value == true, 0);
    }

    fun test_no_schema() {
        UNUSED_Test[@0x1];
    }

    fun test_no_schema_ref() {
        use 0x42::test;
        &test::UNUSED_Test[@0x1];
    }

}
