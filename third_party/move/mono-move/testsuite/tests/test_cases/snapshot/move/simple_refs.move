module 0xc0ffee::refs {
    fun add_refs(x: &u64, y: &u64): u64 {
        *x + *y
    }

    fun add(x: u64, y: u64): u64 {
        add_refs(&x, &y)
    }
}
