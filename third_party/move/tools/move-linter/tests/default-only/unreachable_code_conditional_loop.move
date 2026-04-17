module 0xc0ffee::m {
    public fun noop() {}

    public fun test(p: bool, q: bool) {
        while (p) {
            if (q) {
                loop {};
                noop();    // dead region 1
                noop();
            } else {
                break;
            };
            noop();        // dead region 2
            noop();
        }
    }
}
