module 0xc0ffee::m {
    fun inc(x: u64): u64 {
        x + 1
    }

    public fun test() {
        let x = 0;
        loop {
            x = inc(x);
            if (x > 10) {
                break;
            }
        }
    }
}
