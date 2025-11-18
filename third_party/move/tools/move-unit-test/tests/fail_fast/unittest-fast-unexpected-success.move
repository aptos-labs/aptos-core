address 0x1 {
module UnitTest {
    const ONE: u64 = 0x20001;
    const TWO: u64 = 0x20002;

    public fun bar(rv: u64): u64 {
        rv
    }

    #[test]
    fun test_baz() {
        let ret = bar(17);
        assert!(ret == 17, ONE);
    }

    // we expect a failure, but the assert passes, the fail-fast should trigger here
    #[test, expected_failure(abort_code = TWO)]
    fun test_unexpected_success() {
        let ret = bar(19);
        assert!(ret == 19, TWO);
    }

    #[test]
    fun test_zzz() {
        let ret = bar(17);
        assert!(ret == 17, ONE);
    }

}
}
