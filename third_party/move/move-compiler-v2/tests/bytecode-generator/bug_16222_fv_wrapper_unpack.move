module 0xc0ffee::m {
    package struct Lazy(||) has drop;

    public fun make_lazy(): Lazy {
        Lazy(|| {})
    }

}

module 0xc0ffee::m_friend {
    friend 0xc0ffee::n;
    friend struct Lazy(||) has drop;

    public fun make_lazy(): Lazy {
        Lazy(|| {})
    }
}

module 0xc0ffee::n {
    public fun test() {
        let l = 0xc0ffee::m::make_lazy();
        l();
    }

    public fun test_friend() {
        let l = 0xc0ffee::m_friend::make_lazy();
        l();
    }
}
