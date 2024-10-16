module 0xc0ffee::m {
    fun test() {
        let x = 3;
        let r = &mut x;
        let y = &mut x;
        let _ = freeze(y);
        *r = 4;
    }
}
