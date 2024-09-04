module 0xc0ffee::m {
    fun one(): u64 {
        1
    }

    fun foo(x: u64): u64 {
        x + 1
    }

    public fun test(): u64 {
        let x = one();
        x = foo(x);
        x
    }
}
