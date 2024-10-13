module 0xc0ffee::m {
    struct S {
        x: u64,
    }

    fun consume1(_a: &S, _b: u64, _c: u64, _d: u64, _e: u64, _f: u64) {}

    fun consume2(_a: &S, _b: u64, _c: u64, _d: &S, _e: u64, _f: u64) {}

    fun consume3(_a: &S, _b: u64, _c: u64, _d: u64, _e: &u64, _f: u64) {}

    public fun test01(a: &S) {
        consume1(a, 1, 2, 3, 4, 5);
    }

    public fun test02(a: &S) {
        consume2(a, 1, 2, a, 4, 5);
    }

    public fun test03(a: &S) {
        consume1(a, 1, 2, 3, 4, 5);
        consume1(a, 1, 2, 3, 4, 5);
    }

    public fun test04(a: &S) {
        consume3(a, 1, 2, 3, &a.x, 5);
        consume1(a, 1, 2, 3, a.x, 5);
    }
}
