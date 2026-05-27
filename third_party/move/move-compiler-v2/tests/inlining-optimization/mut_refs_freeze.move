module 0xc0ffee::m {
    fun read(r: &u64): u64 {
        *r
    }

    fun read_then_write(r: &mut u64) {
        let v = read(r);
        *r = v + 1;
    }
}
