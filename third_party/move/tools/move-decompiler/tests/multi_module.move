module 0xc0ffee::m {
    public fun foo(): u64 {
        1
    }
}

module 0xc0ffee::n {
    fun bar(): u64 {
        0xc0ffee::m::foo()
    }
}
