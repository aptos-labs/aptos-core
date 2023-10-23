module 0x42::constant {
    fun test_constans() {
        let const_true = true;
        let const_false = false;
        let hex_u8: u8 = 0x1;
        let hex_u16: u16 = 0x1BAE;
        let hex_u32: u32 = 0xDEAD80;
        let hex_u64: u64 = 0xCAFE;
        let hex_u128: u128 = 0xDEADBEEF;
        let hex_u256: u256 = 0x1123_456A_BCDE_F;
        let a = @0x42;
        let vec = vector[1, 2, 3];
        let s = b"Hello!\n";
    }
}
