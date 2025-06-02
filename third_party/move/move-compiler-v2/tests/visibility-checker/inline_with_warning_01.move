module 0xc0ffee::m {
    fun secret(): u64 {
        42
    }

    inline fun inner(): u64 {
        secret() + secret()
    }

    friend fun some_what_inner(): u64 {
        secret() + secret()
    }

    public inline fun outer(): u64 {
        inner() + inner() + some_what_inner()
    }

    fun test() {
        assert!(outer() == 168);
    }
}
