#[test_only]
module std::test_compile_for_testing {
    #[test]
    fun test() {
        assert!(__COMPILE_FOR_TESTING__, 66);
    }
}
