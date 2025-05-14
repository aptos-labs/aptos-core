module 0xc0ffee::m {
    fun warn_1(p: u64): u64 {
        p = 1;
        p
    }

    fun no_warn_1(p: u64): u64 {
        p = p;
        p
    }

    fun no_warn_2(p: u64): u64 {
        p
    }

    fun no_warn_3(p: u64): u64 {
        let x = p;
        x
    }

    fun no_warn_4(p: u64): u64 {
        no_warn_3(p)
    }

    fun warn_2(p: u64) {
        p = 1;
    }

    fun no_warn_5(_p: u64) {
        _p = 1;
    }

    struct S {
        x: u64,
    }

    fun warn_3(s: S, x: u64): u64 {
        S { x } = s;
        x
    }
}
