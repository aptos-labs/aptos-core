module 0xc0ffee::m {
    struct Lazy(||) has drop;

}

module 0xc0ffee::n {
    public fun test(): 0xc0ffee::m::Lazy {
        || {}
    }
}
