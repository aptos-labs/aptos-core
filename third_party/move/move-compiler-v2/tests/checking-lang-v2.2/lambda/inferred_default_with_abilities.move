// Fixes #16405
module 0xc0ffee::m {
    struct S<T>(T) has key;

    fun one(): u64 {
        1
    }

    public fun test(s: &signer) {
        let f = || {
            let a = one();
            let b = one();
            a + b
        };
        move_to(s, S(f));
    }
}
