module 0x42::m {
    struct S has copy, drop { f: u64, g: u64 }

    // bytecode verification succeeds
    fun t0(s: &mut S) {
        let f = &mut s.f;
        *f;
        *s;
    }
}
