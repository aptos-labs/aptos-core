module 0xc0ffee::m {
    public enum Wrapper has drop {
        V1(u64, u64),
        V2(u64),
    }

    public fun make$Wrapper(x: u64): Wrapper {
        Wrapper::V1(x, x + 1)
    }
}
