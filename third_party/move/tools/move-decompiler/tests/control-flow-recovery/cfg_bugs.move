module 0x99::return_test {
    use std::vector;
    public fun contains(e: u64, v: vector<u64>): bool {
        let i = 0;
        let len = vector::length(&v);
        let res = false;
        while (i < len) {
            if (vector::borrow(&v, i) == &e) {
                res = true;
                break
            };
            i = i + 1;
        };
        res
    }
}
