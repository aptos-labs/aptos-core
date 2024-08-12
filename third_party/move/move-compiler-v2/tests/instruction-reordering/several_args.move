module 0xc0ffee::m {
    fun bar(a: &u64, b: u64, c: u64, d: u64, e: u64, f: u64, g: u64): u64 {
        *a + b + c + d + e + f + g
    }

    public fun test(x: &u64): u64 {
        bar(x, 2, 3, 4, 5, 6, 7)
    }
}
