module 0x42::m {
    use 0x1::vector;

    struct R has key {
        data: vector<u64>
    }


    fun f(k: u8, d: &vector<u64>): u64 acquires R {
        let v =
            if (k == 0) {
                &borrow_global<R>(@0x1).data
            } else {
                d
            };
        *vector::borrow(v, 0)
    }
}
