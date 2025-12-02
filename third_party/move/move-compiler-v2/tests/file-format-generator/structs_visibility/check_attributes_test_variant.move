module 0xc0ffee::m {

    package enum Wrapper has drop {
        V1(u64), // same type at the same offset
        V2(u64),
    }

    #[test_variant]
    public fun test_variant_correct(wrapper: &Wrapper): u8 {
        if (wrapper is Wrapper::V1) {
            2
        } else {
            0
        }
    }

}
