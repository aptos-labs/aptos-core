module 0x42::m {

    fun test_helper(): u64 {
        42
    }

    struct TestStruct has drop {
        x: u64
    }

    const TEST_CONST: u64 = 100;

    #[test]
    fun test_all() {
        let s = TestStruct { x: TEST_CONST };
        assert!(test_helper() + s.x == 142, 1);
    }

    public fun regular(): u64 {
        1
    }
}
