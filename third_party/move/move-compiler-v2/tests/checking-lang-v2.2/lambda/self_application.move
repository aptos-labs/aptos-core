// Fixes #16435
module 0xc0ffee::m {
    public fun test() {
        let f = |func| func();
        f(f);
    }
}
