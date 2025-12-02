module 0xc0ffee::m {

    package enum Wrapper has drop {
        V1(u64), // same type at the same offset
        V2(u64),
    }

    #[pack]
    public fun pack_incorrect(x: u64): Wrapper {
        let x = 2;
        x = x + 1;
        Wrapper::V1(x)
    }

}
