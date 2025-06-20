module 0xc0ffee::m {
    friend 0xc0ffee::n;

    public(friend) fun foo(): u64 {
        42
    }
}

module 0xc0ffee::n {
    friend 0xc0ffee::o;

    public(friend) inline fun bar(): u64 {
        0xc0ffee::m::foo() + 0xc0ffee::m::foo()
    }
}

module 0xc0ffee::o {
    fun test(): u64 {
        42
    }
}
