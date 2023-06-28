module 0x100::Test {
    fun ret_2vals(): (bool, bool) { (true, false) }
    fun ret_4vals(x: &u64): (&u64, u8, u128, u32) { (x, 8, 128, 32) }

    fun use_2val_call_result() {
        let (x, y): (bool, bool) = ret_2vals();
        let _t = x || y;
    }
    fun use_4val_call_result() {
        let (a, b, c, d) = ret_4vals(&0);
        let _t1 = *a;
        let _t2 = b;
        let _t3 = c;
        let _t4 = d;
    }
}
