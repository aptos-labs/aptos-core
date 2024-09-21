module 0xc0ffee::m {
    public fun test(): u8 {
        let x = 40;
        let y = move x;
        let _ = x;
        y
    }
}
