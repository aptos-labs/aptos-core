module 0xc0ffee::ret_swap {
    fun swap_back(a: u64, b: u64): (u64, u64) {
        (b, a)
    }

    fun caller(): u64 {
        let (x, _y) = swap_back(11, 22);
        x
    }
}
