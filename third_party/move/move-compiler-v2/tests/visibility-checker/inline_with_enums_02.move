module 0xc0ffee::m {
    enum Wrapper has drop {
        V1(u64),
    }

    public fun make(x: u64): Wrapper {
        Wrapper::V1(x)
    }

    public inline fun use_me_not(): u64 {
        let x = make(22);
        x.0
    }
}

module 0xc0ffee::n {
    use 0xc0ffee::m;

    fun test(): u64 {
        m::use_me_not()
    }
}
