// Decoupling fixture for the three debug_assert configs. `keep` (a #[test] fn)
// exists only when test code is compiled; debug_assert_test_no_debug is the only
// config where `keep` is present and its debug_assert is stripped.
module 0x42::m {
    public fun prod(x: u64) {
        debug_assert!(x > 0);
    }

    #[test]
    fun keep() {
        debug_assert!(1 + 1 == 2);
    }
}
