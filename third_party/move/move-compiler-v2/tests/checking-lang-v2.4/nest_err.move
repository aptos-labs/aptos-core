module 0xc0ffee::m {
    public struct Wrapper(Inner);

    enum Inner {
        V1(u64, u64),
        V2(u64),
    }

    public struct Wrapper2(Inner2);

    friend enum Inner2 {
        V1(u64, u64),
        V2(u64),
    }

}

module 0xc0ffee::m2 {
    use 0xc0ffee::m::Wrapper;
    use 0xc0ffee::m::Wrapper2;
    use 0xc0ffee::m::Inner;
    use 0xc0ffee::m::Inner2;

    fun test() {
        let x = Wrapper(Inner::V1(22, 23));
    }

    fun test2() {
        let x = Wrapper2(Inner2::V1(22, 23));
    }

}
