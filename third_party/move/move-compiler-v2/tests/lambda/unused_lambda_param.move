module 0xc0ffee::m {
    inline fun test(p: u64, f: |u64| u64): u64 {
        f(p)
    }

    fun unused_lambda() {
        test(0, |x| 1);
    }

    fun unused_lambda_suppressed1() {
        test(0, |_x| 1);
    }

    fun unused_lambda_suppressed2() {
        test(0, |_| 1);
    }
}
