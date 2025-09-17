module UnitTest::UnitTest {
    const ONE: u64 = 0x20001;
    const TWO: u64 = 0x20002;

    public fun bar(rv: u64): u64 {
        rv
    }

    #[test]
    fun test_baz() {
        let ret = bar(17);
        assert!(ret == 18, UnitTest::UnitTest::ONE);
    }

    #[test]
    fun test_ba() {
        let ret = bar(17);
        assert!(ret == 19, UnitTest::UnitTest::ONE);
    }

    #[test]
    fun test_bar() {
        let ret = bar(17);
        assert!(ret == 17, UnitTest::UnitTest::ONE);
    }

    #[test, expected_failure(abort_code = UnitTest::UnitTest::TWO)]
    fun test_foo() {
        let ret = bar(19);
        assert!(ret == 17, UnitTest::UnitTest::TWO);
    }
}
