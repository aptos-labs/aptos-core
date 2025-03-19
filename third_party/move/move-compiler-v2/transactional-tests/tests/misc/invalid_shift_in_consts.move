//# publish
module 0xc0ffee::m {
    const C1: u8 = 0 << 8;
    const C2: u8 = 0 >> 8;

    public fun test(): u8 {
        C1 + C2
    }
}
