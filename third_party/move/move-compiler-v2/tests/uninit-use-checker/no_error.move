module 0xc0::m {
    fun foo(p: u64, q: u64): u64 {
        let x = p + q;
        let y = x + 1;
        let z = y + 1;
        z
    }
}
