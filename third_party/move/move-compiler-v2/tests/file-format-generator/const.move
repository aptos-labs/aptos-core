module 0x42::constant {
    fun test_constans() {
        let const_true = u(true);
        let const_false = u(false);
        let hex_u8: u8 = u(0x1);
        let hex_u16: u16 = u(0x1BAE);
        let hex_u32: u32 = u(0xDEAD80);
        let hex_u64: u64 = u(0xCAFE);
        let hex_u128: u128 = u(0xDEADBEEF);
        let hex_u256: u256 = u(0x1123_456A_BCDE_F);
        let a = u(@0x42);
        let vec = u(vector[1, 2, 3]);
        let s = u(b"Hello!\n");
    }

    fun u<T>(x: T): T { x }
}
