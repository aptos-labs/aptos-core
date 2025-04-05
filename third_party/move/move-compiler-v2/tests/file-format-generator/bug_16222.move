module 0xc0ffee::m {
    struct Lazy(||) has drop;

    public fun make_lazy(): Lazy {
        Lazy(|| {})
    }

}

module 0xc0ffee::n {
    public fun test() {
        let l = 0xc0ffee::m::make_lazy();
        l();
    }
}
