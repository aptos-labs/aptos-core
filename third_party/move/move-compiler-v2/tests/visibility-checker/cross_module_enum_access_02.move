module 0xc0ffee::m {
    enum Wrapper has drop {
        V1(u64),
        V2(u64),
    }

    public fun make(x: u64): Wrapper {
        Wrapper::V1(x)
    }
}

module 0xc0ffee::n {
    use 0xc0ffee::m;

    fun test() {
        let x = m::make(22);
        assert!(x is m::Wrapper::V1, 1);
    }
}
