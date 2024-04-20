module 0xc0ffee::m {
    public fun test(): u8 {
        let x = 40;
        let z = 30;
        let y = move x;
        let (_, q) = (x, z);
        y
    }
}
