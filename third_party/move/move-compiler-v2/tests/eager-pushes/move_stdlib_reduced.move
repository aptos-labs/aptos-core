// Test case to showcase https://github.com/velor-chain/velor-core/issues/15339
module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun bar(x: &mut u64, i: u64) {
        *x = i;
    }

    public fun foo(x: &mut u64, len: u64) {
        while (len > 0) {
            bar(x, one());
            len = len - 1;
        };
    }
}
