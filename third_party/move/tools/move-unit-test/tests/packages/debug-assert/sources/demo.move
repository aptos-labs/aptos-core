module demo::demo {
    public fun half(x: u64): u64 {
        debug_assert!(x % 2 == 0);
        x / 2
    }

    // half(3) aborts inside debug_assert! when assertions are on; returns 1 when off.
    #[test]
    fun test_half_odd() {
        assert!(half(3) == 1, 99);
    }
}
