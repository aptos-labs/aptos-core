module 0xc0ffee::m {
    public enum Wrapper has key {
        V1(u64, u64),
        V2(u64),
    }

    package enum Wrapper2 has key {
        V1(u64, u64),
        V2(u64),
    }

}

module 0xc0ffee::m_friend {

    friend struct Wrapper(u64, u64) has key;

}
