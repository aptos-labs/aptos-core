module 0xc0ffee::m {

    package enum Wrapper has drop {
        V1(u64), // same type at the same offset
        V2(u64),
    }

    #[unpack]
    public fun unpack_incorrect(wrapper: Wrapper): u64 {
        let Wrapper::V1(x) = wrapper;
        x + 1
    }

}
