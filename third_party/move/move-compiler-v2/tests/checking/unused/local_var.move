module 0xc0ffee::m {
    public fun test(): u64 {
        let x = 1;
        let x = x + 1;
        let y = 2;
        let y = y + 1;
        x + y
    }
}
