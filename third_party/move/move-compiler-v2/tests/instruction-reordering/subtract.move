module 0xc0ffee::m {
    public fun test(a: u16, b: u16): u32 {
        let r = ((b - a) as u32);
        r
    }

}
