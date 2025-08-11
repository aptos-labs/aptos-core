// fixes #16468
module 0xc0ffee::m {
    fun take(_x: u64) {}

    fun test(s: signer) {
        let f = |_s: signer| take(5); // _s is not used and can't be curried
        f(s);
    }
}
