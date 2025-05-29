module 0xdeadbeef::m {
    package inline fun blah(): u64 {
        0xdeadbeef::n::foo()
    }
}

module 0xdeadbeef::n {
    friend 0xdeadbeef::m;

    public(friend) fun foo(): u64 {
        42
    }
}
