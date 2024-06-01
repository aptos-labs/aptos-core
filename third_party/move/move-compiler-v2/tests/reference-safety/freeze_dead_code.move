module 0x42::n {
    // Currently expected to fail both v1 and v2
    fun test1() {
        let x = 3;
        let n = &mut x;
        let y = &mut x;
        let _m = &mut x;
        freeze(y);
        *n = *n + 1;
    }

    // Currently expected to fail in v2 but succeeds in v1
    fun test2() {
        let x = 3;
        let _n = &mut x;
        let y = &mut x;
        let m = &mut x;
        freeze(y);
        *m = *m + 1;
    }
}
