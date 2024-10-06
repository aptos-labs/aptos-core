module 0x42::test {

    spec module {
        pragma verify = true;
    }

    fun assert_no_spec(x: u64) {
        assert!(x > 815);
    }

    fun assert_with_spec(x: u64) {
        assert!(x > 815);
    }
    spec assert_with_spec  {
        // This will fail
        aborts_if x > 815 with std::error::internal(0) | (0xCA26CBD9BE << 24);
    }
}
