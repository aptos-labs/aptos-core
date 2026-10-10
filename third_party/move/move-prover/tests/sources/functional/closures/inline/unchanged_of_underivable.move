module 0x42::unchanged_of_underivable {
    inline fun apply(f: |u64|, x: u64) {
        f(x);
        spec {
            assert unchanged_of<f>(x);
        };
    }

    fun test() {
        apply(|x| {
            let i = x;
            while (i > 0) i = i - 1;
        }, 1);
    }
}
