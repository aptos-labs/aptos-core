module 0x100::Test {
    public fun ret_6vals(a: u8, b: u16, c: u32, d: u64, e: u128, f: u256): (u8, u16, u32, u64, u128, u256) {
        (a, b, c, d, e, f)
    }
}

script {
    fun main()  {
        let (x1, x2, x3, x4, x5, x6) = 0x100::Test::ret_6vals(1, 2, 3, 4, 5, 6);
        assert!(x1 == 1, 0xf00);
        assert!(x2 == 2, 0xf01);
        assert!(x3 == 3, 0xf02);
        assert!(x4 == 4, 0xf03);
        assert!(x5 == 5, 0xf04);
        assert!(x6 == 6, 0xf05);
    }
}
