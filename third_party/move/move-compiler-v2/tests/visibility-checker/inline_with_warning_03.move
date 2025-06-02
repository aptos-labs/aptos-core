module 0xc0ffee::m {
    friend 0xc0ffee::n;

    fun secret(): u64 {
        42
    }

    inline fun blah(): u64 {
        secret() + secret()
    }

    public(friend) inline fun foo(): u64 {
        blah() + blah()
    }

    fun test() {
        assert!(foo() == 168);
    }
}

module 0xc0ffee::n {}

module 0xc0ffee::o {
    fun secret(): u64 {
        42
    }

    inline fun blah(): u64 {
        secret() + secret()
    }

    public(friend) inline fun foo(): u64 {
        blah() + blah()
    }

    fun test() {
        assert!(foo() == 168);
    }
}
