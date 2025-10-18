module 0xc0ffee::m {
    package struct Lazy(||) has drop;

}

module 0xc0ffee::m_friend {
    friend 0xc0ffee::n;
    friend struct Lazy(||) has drop;

}

module 0xc0ffee::n {
    public fun test(): 0xc0ffee::m::Lazy {
        || {}
    }

    public fun test_friend(): 0xc0ffee::m_friend::Lazy {
        || {}
    }
}
