module 0xc0ffee::m {
    fun caller(): u64 {
        let x = 1;
        let y = 1;
        callee(x, y)
    }

    fun callee(a: u64, _b: u64): u64 {
        let x = 1;
        let y = a + x;
        y
    }
}
