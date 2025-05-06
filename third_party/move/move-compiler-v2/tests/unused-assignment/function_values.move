module 0xc0ffee::m {
    public fun warn_01() {
        let f = |x, y, z: u64| {
            x + y
        };
        let g = |x| |y| |z: u64| x + y + z;
    }

    inline fun warn_02(x: u64) {}

    fun use_inline() {
        warn_02(42);
    }

    struct S {
        x: u64,
    }

    fun test(): u64 {
        let f = |S{x}| 1;
        f(S { x: 42 })
    }

    fun warn_03(f: |&u64| has drop) {
    }

    fun run(p: u64, f: |u64| u64): u64 {
        f(p)
    }

    fun warn_04() {
        run(0, |x| 1);
    }
}
